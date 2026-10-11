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
use inklog_examples::common::{print_section, print_separator};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== inklog health monitoring and metrics example ===\n");

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

    println!("\n✓ All health monitoring and metrics examples completed");
    Ok(())
}

/// 展示 Metrics 创建和基础计数器操作
fn show_metrics_creation() {
    print_separator("1. Metrics creation and basic counters");

    print_section("1.1 Creating an instance with Metrics::new()");
    let metrics = Metrics::new();
    println!("initial logs_written = {}", metrics.logs_written());
    println!("initial logs_dropped = {}", metrics.logs_dropped());
    println!("initial sink_errors  = {}", metrics.sink_errors());
    println!("initial channel_blocked = {}", metrics.channel_blocked());
    assert_eq!(metrics.logs_written(), 0);

    print_section("1.2 inc_logs_written() accumulates log writes");
    for i in 0..5 {
        metrics.inc_logs_written();
        println!(
            "  write #{}, cumulative = {}",
            i + 1,
            metrics.logs_written()
        );
    }
    assert_eq!(metrics.logs_written(), 5);

    print_section("1.3 inc_logs_dropped() / inc_sink_error() / inc_channel_blocked()");
    metrics.inc_logs_dropped();
    metrics.inc_logs_dropped();
    metrics.inc_sink_error();
    metrics.inc_channel_blocked();
    metrics.inc_channel_blocked();
    metrics.inc_channel_blocked();
    println!("logs_dropped   = {}", metrics.logs_dropped());
    println!("sink_errors    = {}", metrics.sink_errors());
    println!("channel_blocked = {}", metrics.channel_blocked());

    print_section("1.4 DB batch metrics set_db_batch_size / add_db_batch_records_total");
    metrics.set_db_batch_size(64);
    metrics.add_db_batch_records_total(64);
    metrics.add_db_batch_records_total(32);
    println!("db_batch_size           = {}", metrics.db_batch_size());
    println!(
        "db_batch_records_total  = {}",
        metrics.db_batch_records_total()
    );
    assert_eq!(metrics.db_batch_size(), 64);
    assert_eq!(metrics.db_batch_records_total(), 96);

    print_section("1.5 Lock contention counter (active_workers is internal; read-only externally)");
    metrics.inc_lock_contention();
    metrics.inc_lock_contention();
    println!(
        "active_workers   = {} (default 0, set by internal workers)",
        metrics.active_workers()
    );
    println!("lock_contention  = {}", metrics.lock_contention());

    print_section("1.6 uptime() runtime duration");
    let uptime = metrics.uptime();
    println!("uptime = {:?}", uptime);
    assert!(uptime.as_secs() < 60);
}

/// 展示延迟直方图与百分位统计
fn show_latency_histogram() {
    print_separator("2. Latency histogram and percentiles");

    print_section("2.1 record_latency() records several latencies");
    let metrics = Metrics::new();
    // 模拟 10 条延迟：50μs ~ 5000μs
    let samples = [50u64, 120, 300, 800, 1500, 2200, 3500, 5000, 7500, 12000];
    for &us in &samples {
        metrics.record_latency(Duration::from_micros(us));
    }
    println!("recorded {} latency samples", samples.len());

    print_section("2.2 Latency aggregates (avg / p50 / p95 / p99)");
    let status = metrics.get_status(0, 100);
    let snap = &status.metrics;
    println!("avg_latency_us = {}", snap.avg_latency_us);
    println!("p50_latency_us = {}", snap.p50_latency_us);
    println!("p95_latency_us = {}", snap.p95_latency_us);
    println!("p99_latency_us = {}", snap.p99_latency_us);
    println!("latency_distribution = {:?}", snap.latency_distribution);
    assert!(snap.p95_latency_us >= snap.p50_latency_us);
}

/// 展示 GaugeF64 浮点指标
fn show_gauge_f64() {
    print_separator("3. GaugeF64 floating-point metric");

    print_section("3.1 GaugeF64::new() / set() / get()");
    let gauge = GaugeF64::new(0.0);
    println!("initial value = {}", gauge.get());
    gauge.set(87.5);
    println!("after set(87.5) = {}", gauge.get());
    gauge.set(99.9);
    println!("after set(99.9) = {}", gauge.get());

    print_section("3.2 The pool_hit_rate metric exposed by Metrics");
    let metrics = Metrics::new();
    metrics.set_pool_hit_rate(85.5);
    println!("pool_hit_rate = {:.2}%", metrics.pool_hit_rate());
    metrics.set_pool_hit_rate(0.0);
    println!(
        "pool_hit_rate after reset = {:.2}%",
        metrics.pool_hit_rate()
    );
    assert!((metrics.pool_hit_rate() - 0.0).abs() < f64::EPSILON);

    print_section("3.3 SinkHealth::healthy() / unhealthy() constructors");
    let healthy = SinkHealth::healthy();
    assert!(matches!(healthy.status, SinkStatus::Healthy));
    assert_eq!(healthy.consecutive_failures, 0);
    println!("healthy.status = {:?}", healthy.status);

    let unhealthy = SinkHealth::unhealthy("Connection refused".to_string());
    assert!(matches!(unhealthy.status, SinkStatus::Unhealthy { .. }));
    assert_eq!(unhealthy.consecutive_failures, 1);
    println!("unhealthy.status = {:?}", unhealthy.status);
}

/// 展示 SinkStatus 状态转换
fn show_sink_status_transitions() {
    print_separator("4. SinkStatus transitions");

    print_section("4.1 Default state NotStarted");
    let mut status = SinkStatus::default();
    println!("status = {:?}", status);
    assert_eq!(status, SinkStatus::NotStarted);
    assert!(!status.is_operational());

    print_section("4.2 NotStarted → Healthy");
    status = SinkStatus::Healthy;
    println!("status = {:?}", status);
    assert!(status.is_operational());

    print_section("4.3 Healthy → Degraded { reason }");
    status = SinkStatus::Degraded {
        reason: "slow disk IO".to_string(),
    };
    println!("status = {:?}", status);
    assert!(status.is_operational(), "Degraded still runs");

    print_section("4.4 Degraded → Unhealthy { error }");
    status = SinkStatus::Unhealthy {
        error: "disk space low".to_string(),
    };
    println!("status = {:?}", status);
    assert!(!status.is_operational());

    print_section("4.5 Unhealthy → Healthy (recovery)");
    status = SinkStatus::Healthy;
    println!("status = {:?}", status);
    assert!(status.is_operational());

    print_section("4.6 State machine summary");
    println!(
        "NotStarted.is_operational()  = {}",
        SinkStatus::NotStarted.is_operational()
    );
    println!(
        "Healthy.is_operational()     = {}",
        SinkStatus::Healthy.is_operational()
    );
    println!(
        "Degraded.is_operational()    = {}",
        SinkStatus::Degraded { reason: "r".into() }.is_operational()
    );
    println!(
        "Unhealthy.is_operational()   = {}",
        SinkStatus::Unhealthy { error: "e".into() }.is_operational()
    );
}

/// 展示 Metrics 的 Sink 健康跟踪
fn show_sink_health_tracking() {
    print_separator("5. Metrics sink health tracking");

    print_section("5.1 sink_started() marks a sink as started");
    let metrics = Metrics::new();
    metrics.sink_started("console");
    metrics.sink_started("file");
    metrics.sink_started("database");
    let health = metrics.sink_health();
    for name in ["console", "file", "database"] {
        let h = health.get(name).expect("sink should exist");
        println!(
            "  {} -> {:?}, failures = {}",
            name, h.status, h.consecutive_failures
        );
        assert!(matches!(h.status, SinkStatus::Healthy));
    }

    print_section("5.2 sink_degraded() marks degradation");
    metrics.sink_degraded("file", "disk IO latency increased".to_string());
    let health = metrics.sink_health();
    let h = health.get("file").expect("file should exist");
    println!("file.status = {:?}", h.status);
    println!("file.last_error = {:?}", h.last_error);
    assert!(matches!(h.status, SinkStatus::Degraded { .. }));

    print_section("5.3 update_sink_health() accumulates the failure count");
    // 多次失败，consecutive_failures 应累加
    metrics.update_sink_health("database", false, Some("Connection refused".into()));
    metrics.update_sink_health("database", false, Some("Timeout".into()));
    let health = metrics.sink_health();
    let h = health.get("database").expect("database should exist");
    println!("database.status = {:?}", h.status);
    println!("database.consecutive_failures = {}", h.consecutive_failures);
    assert_eq!(h.consecutive_failures, 2);

    print_section("5.4 update_sink_health() resets the count on recovery");
    metrics.update_sink_health("database", true, None);
    let health = metrics.sink_health();
    let h = health.get("database").expect("database should exist");
    println!("database.status = {:?}", h.status);
    println!("database.consecutive_failures = {}", h.consecutive_failures);
    assert_eq!(h.consecutive_failures, 0);

    print_section("5.5 update_sink_health() uses a default message when no error info exists");
    metrics.update_sink_health("file", false, None);
    let health = metrics.sink_health();
    let h = health.get("file").expect("file should exist");
    if let SinkStatus::Unhealthy { error } = &h.status {
        println!("file.status.error = {:?}", error);
        assert_eq!(error, "Unknown error");
    } else {
        panic!("file should be Unhealthy");
    }
}

/// 展示 HealthStatus 整体快照
fn show_health_status_snapshot() {
    print_separator("6. HealthStatus overall snapshot");

    print_section("6.1 Empty sinks → NotStarted");
    let metrics = Metrics::new();
    let status: HealthStatus = metrics.get_status(0, 100);
    println!("overall_status = {:?}", status.overall_status);
    assert!(matches!(status.overall_status, SinkStatus::NotStarted));

    print_section("6.2 All Healthy");
    metrics.sink_started("console");
    metrics.sink_started("file");
    let status = metrics.get_status(50, 1000);
    println!("overall_status = {:?}", status.overall_status);
    println!("channel_usage  = {:.2}%", status.channel_usage * 100.0);
    println!("uptime_seconds = {}", status.uptime_seconds);
    println!("encryption_key_valid = {}", status.encryption_key_valid);
    assert!(matches!(status.overall_status, SinkStatus::Healthy));

    print_section("6.3 Degraded present (no Unhealthy)");
    metrics.sink_degraded("file", "slow IO".to_string());
    let status = metrics.get_status(0, 100);
    println!("overall_status = {:?}", status.overall_status);
    assert!(matches!(status.overall_status, SinkStatus::Degraded { .. }));

    print_section("6.4 Unhealthy present");
    metrics.update_sink_health("database", false, Some("Connection refused".into()));
    let status = metrics.get_status(0, 100);
    println!("overall_status = {:?}", status.overall_status);
    match &status.overall_status {
        SinkStatus::Unhealthy { error } => {
            println!("error = {}", error);
            assert!(error.contains("Connection refused"));
        }
        _ => panic!("should be Unhealthy"),
    }

    print_section("6.5 HealthStatus fields at a glance");
    println!("sinks count = {}", status.sinks.len());
    println!("metrics.logs_written     = {}", status.metrics.logs_written);
    println!("metrics.sink_errors      = {}", status.metrics.sink_errors);
    println!(
        "metrics.active_workers   = {}",
        status.metrics.active_workers
    );
    println!(
        "metrics.pool_hit_rate    = {}",
        status.metrics.pool_hit_rate
    );
    println!("pool_stats = {:?}", status.pool_stats);
}

/// 展示 SinkHealthMonitor 智能降级与恢复
fn show_sink_health_monitor() {
    print_separator("7. SinkHealthMonitor smart fallback and recovery");

    print_section("7.1 SinkHealthMonitor::default() initial state");
    let monitor = SinkHealthMonitor::default();
    println!(
        "database initial state = {:?}",
        monitor.get_fallback_state("database")
    );
    assert_eq!(
        monitor.get_fallback_state("database"),
        FallbackState::Active
    );
    assert!(!monitor.is_any_in_fallback());

    print_section("7.2 Failures below threshold → Retry");
    let action = monitor.check_and_fallback("database", false, Some("Connection refused"));
    println!("action = {:?}", action);
    assert!(matches!(action, FallbackAction::Retry { .. }));
    println!("requires_action = {}", action.requires_action());
    println!("sink_name = {:?}", action.sink_name());

    print_section("7.3 Threshold reached → Fallback to file");
    // 默认 failure_threshold = 3，前面已失败 1 次
    for _ in 0..3 {
        let _ = monitor.check_and_fallback("database", false, Some("Connection refused"));
    }
    let state = monitor.get_fallback_state("database");
    println!("database state = {:?}", state);
    match &state {
        FallbackState::Fallback { target, reason } => {
            println!("target = {}", target);
            println!("reason = {}", reason);
            assert_eq!(target, "file");
        }
        _ => panic!("should be Fallback"),
    }
    assert!(monitor.is_any_in_fallback());

    print_section("7.4 Healthy again → AttemptRecovery (exponential backoff)");
    let action = monitor.check_and_fallback("database", true, None);
    println!("action = {:?}", action);
    match &action {
        FallbackAction::AttemptRecovery { delay_ms, .. } => {
            println!("delay_ms = {}", delay_ms);
        }
        _ => panic!("should be AttemptRecovery"),
    }

    print_section("7.5 Healthy again during recovery → Wait");
    let action = monitor.check_and_fallback("database", true, None);
    println!("action = {:?}", action);
    assert!(matches!(action, FallbackAction::Wait { .. }));

    print_section("7.6 confirm_recovery() confirms recovery");
    monitor.confirm_recovery("database");
    println!(
        "database state = {:?}",
        monitor.get_fallback_state("database")
    );
    assert_eq!(
        monitor.get_fallback_state("database"),
        FallbackState::Active
    );

    print_section("7.7 FileSink disk full → fallback to console");
    let monitor2 = SinkHealthMonitor::with_defaults();
    for _ in 0..3 {
        let _ = monitor2.check_and_fallback("file", false, Some("No space left on device"));
    }
    let state = monitor2.get_fallback_state("file");
    println!("file state = {:?}", state);
    match &state {
        FallbackState::Fallback { target, .. } => assert_eq!(target, "console"),
        _ => panic!("should be Fallback"),
    }

    print_section("7.8 Encryption error → fallback to plaintext");
    let monitor3 = SinkHealthMonitor::with_defaults();
    let action = monitor3.handle_encryption_error("file", "Invalid key");
    println!("action = {:?}", action);
    match &action {
        FallbackAction::Fallback { target, .. } => assert_eq!(target, "plaintext"),
        _ => panic!("should be Fallback"),
    }

    print_section("7.9 Custom FallbackConfig + FallbackStats");
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
    println!("stats = {:?}", stats);
    assert_eq!(stats.active_fallbacks, 0);

    print_section("7.10 reset() resets the monitor");
    for _ in 0..3 {
        let _ = monitor4.check_and_fallback("database", false, Some("err"));
    }
    assert!(monitor4.is_any_in_fallback());
    monitor4.reset();
    assert!(!monitor4.is_any_in_fallback());
    println!(
        "is_any_in_fallback after reset = {}",
        monitor4.is_any_in_fallback()
    );

    print_section("7.11 get_fallback_events() event history");
    let monitor5 = SinkHealthMonitor::with_defaults();
    for _ in 0..3 {
        let _ = monitor5.check_and_fallback("database", false, Some("err"));
    }
    let events = monitor5.get_fallback_events(10);
    println!("event count = {}", events.len());
    assert!(!events.is_empty());
    println!("first event sink = {}", events[0].sink_name);
    println!("first event from = {:?}", events[0].from_state);
    println!("first event to   = {:?}", events[0].to_state);
}

/// 展示 Prometheus 格式导出
fn show_prometheus_export() {
    print_separator("8. Prometheus format export");

    print_section("8.1 Simulated metric collection");
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

    print_section("8.2 export_prometheus() output (key lines excerpted)");
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

    print_section("8.3 HealthStatus serialization preview (serde_json)");
    let status = metrics.get_status(128, 1024);
    let json = serde_json::to_string_pretty(&status.metrics).unwrap_or_default();
    // 仅打印前若干行避免刷屏
    for (i, line) in json.lines().enumerate() {
        if i >= 12 {
            println!("  ... (truncated)");
            break;
        }
        println!("  {}", line);
    }

    println!(
        "\n✓ Prometheus export has {} characters, {} lines",
        output.len(),
        output.lines().count()
    );
}
