// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use inklog::support::observability::metrics::FallbackAction;
use inklog::support::observability::{
	FallbackConfig, FallbackState, GaugeF64, HealthStatus, Metrics, SinkHealth, SinkHealthMonitor,
	SinkStatus,
};
use inklog_examples::common::{print_section, print_separator};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== inklog 健康监控与指标示例 ===\n");

	// 1. Metrics 创建与基础计数器
	show_metrics_creation();

	// 2. 延迟直方图与百分位数
	show_latency_histogram();

	// 3. GaugeF64 浮点指标
	show_gauge_f64();

	// 4. SinkStatus 状态转换
	show_sink_status_transitions();

	// 5. Sink 健康监控（Metrics.update_sink_health）
	show_sink_health_tracking();

	// 6. HealthStatus 整体快照
	show_health_status_snapshot();

	// 7. SinkHealthMonitor 智能降级与恢复
	show_sink_health_monitor();

	// 8. Prometheus 格式导出
	show_prometheus_export();

	println!("\n✓ 所有健康监控与指标示例演示完成");
	Ok(())
}

/// 展示 Metrics 创建和基础计数器操作
fn show_metrics_creation() {
	print_separator("1. Metrics 创建与基础计数器");

	print_section("1.1 Metrics::new() 创建实例");
	let metrics = Metrics::new();
	println!("初始 logs_written = {}", metrics.logs_written());
	println!("初始 logs_dropped = {}", metrics.logs_dropped());
	println!("初始 sink_errors  = {}", metrics.sink_errors());
	println!("初始 channel_blocked = {}", metrics.channel_blocked());
	assert_eq!(metrics.logs_written(), 0);

	print_section("1.2 inc_logs_written() 累加日志写入");
	for i in 0..5 {
		metrics.inc_logs_written();
		println!("  第 {} 次写入，累计 = {}", i + 1, metrics.logs_written());
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

	print_section("1.4 DB 批量指标 set_db_batch_size / add_db_batch_records_total");
	metrics.set_db_batch_size(64);
	metrics.add_db_batch_records_total(64);
	metrics.add_db_batch_records_total(32);
	println!("db_batch_size           = {}", metrics.db_batch_size());
	println!("db_batch_records_total  = {}", metrics.db_batch_records_total());
	assert_eq!(metrics.db_batch_size(), 64);
	assert_eq!(metrics.db_batch_records_total(), 96);

	print_section("1.5 锁竞争计数（active_workers 为内部字段，外部只读）");
	metrics.inc_lock_contention();
	metrics.inc_lock_contention();
	println!("active_workers   = {}（默认 0，由内部 worker 设置）", metrics.active_workers());
	println!("lock_contention  = {}", metrics.lock_contention());

	print_section("1.6 uptime() 运行时长");
	let uptime = metrics.uptime();
	println!("uptime = {:?}", uptime);
	assert!(uptime.as_secs() < 60);
}

/// 展示延迟直方图与百分位统计
fn show_latency_histogram() {
	print_separator("2. 延迟直方图与百分位数");

	print_section("2.1 record_latency() 记录多条延迟");
	let metrics = Metrics::new();
	// 模拟 10 条延迟：50μs ~ 5000μs
	let samples = [50u64, 120, 300, 800, 1500, 2200, 3500, 5000, 7500, 12000];
	for &us in &samples {
		metrics.record_latency(Duration::from_micros(us));
	}
	println!("已记录 {} 条延迟样本", samples.len());

	print_section("2.2 延迟聚合统计（avg / p50 / p95 / p99）");
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
	print_separator("3. GaugeF64 浮点指标");

	print_section("3.1 GaugeF64::new() / set() / get()");
	let gauge = GaugeF64::new(0.0);
	println!("初始值 = {}", gauge.get());
	gauge.set(87.5);
	println!("set(87.5) 后 = {}", gauge.get());
	gauge.set(99.9);
	println!("set(99.9) 后 = {}", gauge.get());

	print_section("3.2 Metrics 暴露的 pool_hit_rate 指标");
	let metrics = Metrics::new();
	metrics.set_pool_hit_rate(85.5);
	println!("pool_hit_rate = {:.2}%", metrics.pool_hit_rate());
	metrics.set_pool_hit_rate(0.0);
	println!("重置后 pool_hit_rate = {:.2}%", metrics.pool_hit_rate());
	assert!((metrics.pool_hit_rate() - 0.0).abs() < f64::EPSILON);

	print_section("3.3 SinkHealth::healthy() / unhealthy() 构造");
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
	print_separator("4. SinkStatus 状态转换");

	print_section("4.1 默认状态 NotStarted");
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
		reason: "磁盘 IO 缓慢".to_string(),
	};
	println!("status = {:?}", status);
	assert!(status.is_operational(), "Degraded 仍可运行");

	print_section("4.4 Degraded → Unhealthy { error }");
	status = SinkStatus::Unhealthy {
		error: "磁盘空间不足".to_string(),
	};
	println!("status = {:?}", status);
	assert!(!status.is_operational());

	print_section("4.5 Unhealthy → Healthy（恢复）");
	status = SinkStatus::Healthy;
	println!("status = {:?}", status);
	assert!(status.is_operational());

	print_section("4.6 状态机汇总");
	println!("NotStarted.is_operational()  = {}", SinkStatus::NotStarted.is_operational());
	println!("Healthy.is_operational()     = {}", SinkStatus::Healthy.is_operational());
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
	print_separator("5. Metrics Sink 健康跟踪");

	print_section("5.1 sink_started() 标记 Sink 启动");
	let metrics = Metrics::new();
	metrics.sink_started("console");
	metrics.sink_started("file");
	metrics.sink_started("database");
	let health = metrics.sink_health();
	for name in ["console", "file", "database"] {
		let h = health.get(name).expect("sink 应存在");
		println!("  {} -> {:?}, failures = {}", name, h.status, h.consecutive_failures);
		assert!(matches!(h.status, SinkStatus::Healthy));
	}

	print_section("5.2 sink_degraded() 标记降级");
	metrics.sink_degraded("file", "磁盘 IO 延迟升高".to_string());
	let health = metrics.sink_health();
	let h = health.get("file").expect("file 应存在");
	println!("file.status = {:?}", h.status);
	println!("file.last_error = {:?}", h.last_error);
	assert!(matches!(h.status, SinkStatus::Degraded { .. }));

	print_section("5.3 update_sink_health() 累积失败计数");
	// 多次失败，consecutive_failures 应累加
	metrics.update_sink_health("database", false, Some("Connection refused".into()));
	metrics.update_sink_health("database", false, Some("Timeout".into()));
	let health = metrics.sink_health();
	let h = health.get("database").expect("database 应存在");
	println!("database.status = {:?}", h.status);
	println!("database.consecutive_failures = {}", h.consecutive_failures);
	assert_eq!(h.consecutive_failures, 2);

	print_section("5.4 update_sink_health() 恢复健康重置计数");
	metrics.update_sink_health("database", true, None);
	let health = metrics.sink_health();
	let h = health.get("database").expect("database 应存在");
	println!("database.status = {:?}", h.status);
	println!("database.consecutive_failures = {}", h.consecutive_failures);
	assert_eq!(h.consecutive_failures, 0);

	print_section("5.5 update_sink_health() 无错误信息时使用默认消息");
	metrics.update_sink_health("file", false, None);
	let health = metrics.sink_health();
	let h = health.get("file").expect("file 应存在");
	if let SinkStatus::Unhealthy { error } = &h.status {
		println!("file.status.error = {:?}", error);
		assert_eq!(error, "Unknown error");
	} else {
		panic!("file 应处于 Unhealthy 状态");
	}
}

/// 展示 HealthStatus 整体快照
fn show_health_status_snapshot() {
	print_separator("6. HealthStatus 整体快照");

	print_section("6.1 空 sinks → NotStarted");
	let metrics = Metrics::new();
	let status: HealthStatus = metrics.get_status(0, 100);
	println!("overall_status = {:?}", status.overall_status);
	assert!(matches!(status.overall_status, SinkStatus::NotStarted));

	print_section("6.2 全部 Healthy");
	metrics.sink_started("console");
	metrics.sink_started("file");
	let status = metrics.get_status(50, 1000);
	println!("overall_status = {:?}", status.overall_status);
	println!("channel_usage  = {:.2}%", status.channel_usage * 100.0);
	println!("uptime_seconds = {}", status.uptime_seconds);
	println!("encryption_key_valid = {}", status.encryption_key_valid);
	assert!(matches!(status.overall_status, SinkStatus::Healthy));

	print_section("6.3 存在 Degraded（无 Unhealthy）");
	metrics.sink_degraded("file", "IO 缓慢".to_string());
	let status = metrics.get_status(0, 100);
	println!("overall_status = {:?}", status.overall_status);
	assert!(matches!(status.overall_status, SinkStatus::Degraded { .. }));

	print_section("6.4 存在 Unhealthy");
	metrics.update_sink_health("database", false, Some("Connection refused".into()));
	let status = metrics.get_status(0, 100);
	println!("overall_status = {:?}", status.overall_status);
	match &status.overall_status {
		SinkStatus::Unhealthy { error } => {
			println!("error = {}", error);
			assert!(error.contains("Connection refused"));
		}
		_ => panic!("应为 Unhealthy"),
	}

	print_section("6.5 HealthStatus 字段一览");
	println!("sinks 数量 = {}", status.sinks.len());
	println!("metrics.logs_written     = {}", status.metrics.logs_written);
	println!("metrics.sink_errors      = {}", status.metrics.sink_errors);
	println!("metrics.active_workers   = {}", status.metrics.active_workers);
	println!("metrics.pool_hit_rate    = {}", status.metrics.pool_hit_rate);
	println!("pool_stats = {:?}", status.pool_stats);
}

/// 展示 SinkHealthMonitor 智能降级与恢复
fn show_sink_health_monitor() {
	print_separator("7. SinkHealthMonitor 智能降级与恢复");

	print_section("7.1 SinkHealthMonitor::default() 初始状态");
	let monitor = SinkHealthMonitor::default();
	println!("database 初始状态 = {:?}", monitor.get_fallback_state("database"));
	assert_eq!(monitor.get_fallback_state("database"), FallbackState::Active);
	assert!(!monitor.is_any_in_fallback());

	print_section("7.2 故障未达阈值 → Retry");
	let action = monitor.check_and_fallback("database", false, Some("Connection refused"));
	println!("action = {:?}", action);
	assert!(matches!(action, FallbackAction::Retry { .. }));
	println!("requires_action = {}", action.requires_action());
	println!("sink_name = {:?}", action.sink_name());

	print_section("7.3 达到阈值 → Fallback 到 file");
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
		_ => panic!("应处于 Fallback 状态"),
	}
	assert!(monitor.is_any_in_fallback());

	print_section("7.4 恢复健康 → AttemptRecovery（指数退避）");
	let action = monitor.check_and_fallback("database", true, None);
	println!("action = {:?}", action);
	match &action {
		FallbackAction::AttemptRecovery { delay_ms, .. } => {
			println!("delay_ms = {}", delay_ms);
		}
		_ => panic!("应为 AttemptRecovery"),
	}

	print_section("7.5 恢复中再次健康 → Wait");
	let action = monitor.check_and_fallback("database", true, None);
	println!("action = {:?}", action);
	assert!(matches!(action, FallbackAction::Wait { .. }));

	print_section("7.6 confirm_recovery() 确认恢复");
	monitor.confirm_recovery("database");
	println!("database state = {:?}", monitor.get_fallback_state("database"));
	assert_eq!(monitor.get_fallback_state("database"), FallbackState::Active);

	print_section("7.7 FileSink 磁盘满 → 降级到 console");
	let monitor2 = SinkHealthMonitor::with_defaults();
	for _ in 0..3 {
		let _ = monitor2.check_and_fallback("file", false, Some("No space left on device"));
	}
	let state = monitor2.get_fallback_state("file");
	println!("file state = {:?}", state);
	match &state {
		FallbackState::Fallback { target, .. } => assert_eq!(target, "console"),
		_ => panic!("应处于 Fallback 状态"),
	}

	print_section("7.8 加密错误 → 降级到 plaintext");
	let monitor3 = SinkHealthMonitor::with_defaults();
	let action = monitor3.handle_encryption_error("file", "Invalid key");
	println!("action = {:?}", action);
	match &action {
		FallbackAction::Fallback { target, .. } => assert_eq!(target, "plaintext"),
		_ => panic!("应为 Fallback"),
	}

	print_section("7.9 FallbackConfig 自定义 + FallbackStats 统计");
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

	print_section("7.10 reset() 重置监控器");
	for _ in 0..3 {
		let _ = monitor4.check_and_fallback("database", false, Some("err"));
	}
	assert!(monitor4.is_any_in_fallback());
	monitor4.reset();
	assert!(!monitor4.is_any_in_fallback());
	println!("reset 后 is_any_in_fallback = {}", monitor4.is_any_in_fallback());

	print_section("7.11 get_fallback_events() 事件历史");
	let monitor5 = SinkHealthMonitor::with_defaults();
	for _ in 0..3 {
		let _ = monitor5.check_and_fallback("database", false, Some("err"));
	}
	let events = monitor5.get_fallback_events(10);
	println!("事件数 = {}", events.len());
	assert!(!events.is_empty());
	println!("首条事件 sink = {}", events[0].sink_name);
	println!("首条事件 from = {:?}", events[0].from_state);
	println!("首条事件 to   = {:?}", events[0].to_state);
}

/// 展示 Prometheus 格式导出
fn show_prometheus_export() {
	print_separator("8. Prometheus 格式导出");

	print_section("8.1 模拟指标采集");
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
	// 设置 Sink 健康
	metrics.sink_started("console");
	metrics.sink_started("file");
	metrics.sink_started("database");
	metrics.update_sink_health("database", false, Some("Connection refused".into()));

	print_section("8.2 export_prometheus() 输出（截取关键行）");
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
	assert!(output.contains("# TYPE inklog_latency_bucket counter"));
	assert!(output.contains("inklog_latency_bucket{le=\"+Inf\"}"));

	// 打印部分输出（避免刷屏）
	for line in output.lines() {
		if line.starts_with("inklog_") && !line.starts_with("inklog_latency_bucket") {
			println!("  {}", line);
		}
	}

	print_section("8.3 HealthStatus 序列化预览（serde_json）");
	let status = metrics.get_status(128, 1024);
	let json = serde_json::to_string_pretty(&status.metrics).unwrap_or_default();
	// 仅打印前若干行避免刷屏
	for (i, line) in json.lines().enumerate() {
		if i >= 12 {
			println!("  ...（已截断）");
			break;
		}
		println!("  {}", line);
	}

	println!("\n✓ Prometheus 导出包含 {} 个字符，{} 行", output.len(), output.lines().count());
}
