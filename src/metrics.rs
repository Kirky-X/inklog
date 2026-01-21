// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # 健康监控模块
//!
//! 提供 Inklog 的健康监控和指标收集功能，支持 Prometheus 格式导出。
//!
//! ## 概述
//!
//! 此模块包含：
//! - **SinkStatus/SinkHealth**：Sink 组件健康状态跟踪
//! - **Metrics**：核心指标收集器
//! - **Prometheus 导出**：HTTP 端点可读格式
//!
// ## 功能特性
//!
//! - **实时健康检查**：跟踪各 sink 的运行状态
//! - **指标收集**：记录日志写入、错误、延迟等
//! - **直方图统计**：延迟分布统计
//! - **Prometheus 兼容**：可直接与 Prometheus 集成
//!
// ## 使用示例
//!
//! ```rust
//! use inklog::metrics::{Metrics, SinkHealth, SinkStatus};
//!
//! let metrics = Metrics::new();
//!
//! // 记录日志写入
//! metrics.inc_logs_written();
//!
//! // 记录错误
//! metrics.inc_sink_error();
//!
//! // 更新 Sink 健康状态
//! metrics.update_sink_health("console", true, None);
//!
//! // 获取整体健康状态
//! let health = metrics.get_status(0, 10000);
//! ```
//!
//! ## Prometheus 指标
//!
//! | 指标 | 类型 | 描述 |
//! |------|------|------|
//! | `inklog_records_total` | Counter | 总日志记录数 |
//! | `inklog_errors_total` | Counter | 总错误数 |
//! | `inklog_latency_us` | Histogram | 处理延迟（微秒）|
//! | `inklog_sink_healthy` | Gauge | Sink 健康状态 |
//! | `inklog_uptime_seconds` | Gauge | 运行时间（秒）|

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Represents the health status of a sink component.
///
/// This enum provides more granular status information than a simple boolean,
/// allowing for better observability and debugging.
#[derive(Debug, Serialize, Clone, PartialEq, Default)]
pub enum SinkStatus {
    /// Sink is operating normally
    Healthy,
    /// Sink is degraded but still functioning
    Degraded { reason: String },
    /// Sink has failed and is not functioning
    Unhealthy { error: String },
    #[default]
    /// Sink has not been initialized yet
    NotStarted,
}

impl SinkStatus {
    /// Returns true if the sink is operational (healthy or degraded but functional)
    pub fn is_operational(&self) -> bool {
        match self {
            SinkStatus::Healthy => true,
            SinkStatus::Degraded { .. } => true,
            SinkStatus::Unhealthy { .. } => false,
            SinkStatus::NotStarted => false,
        }
    }

    /// Returns true if the sink is completely healthy with no issues
    fn is_fully_healthy(&self) -> bool {
        self == &SinkStatus::Healthy
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct SinkHealth {
    pub status: SinkStatus,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}

impl Default for SinkHealth {
    fn default() -> Self {
        Self {
            status: SinkStatus::NotStarted,
            last_error: None,
            consecutive_failures: 0,
        }
    }
}

impl SinkHealth {
    /// Creates a healthy sink status
    pub fn healthy() -> Self {
        Self {
            status: SinkStatus::Healthy,
            last_error: None,
            consecutive_failures: 0,
        }
    }

    /// Creates an unhealthy sink status with the given error
    pub fn unhealthy(error: String) -> Self {
        Self {
            status: SinkStatus::Unhealthy {
                error: error.clone(),
            },
            last_error: Some(error),
            consecutive_failures: 1,
        }
    }
}

/// Gauge metric for atomic counter values
#[derive(Debug)]
pub struct Gauge {
    value: AtomicI64,
}

impl Gauge {
    pub fn new(val: i64) -> Self {
        Self {
            value: AtomicI64::new(val),
        }
    }
    pub fn set(&self, v: i64) {
        self.value.store(v, Ordering::Relaxed);
    }
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Histogram metric for latency distribution
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<AtomicU64>,
    bounds: Vec<u64>, // in microseconds
}

impl Histogram {
    pub fn new(bounds: Vec<u64>) -> Self {
        let mut buckets = Vec::with_capacity(bounds.len() + 1);
        for _ in 0..=bounds.len() {
            buckets.push(AtomicU64::new(0));
        }
        Self { buckets, bounds }
    }

    pub fn record(&self, value: u64) {
        let mut index = self.bounds.len();
        for (i, &bound) in self.bounds.iter().enumerate() {
            if value < bound {
                index = i;
                break;
            }
        }
        self.buckets[index].fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Vec<u64> {
        self.buckets
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub logs_written: u64,
    pub logs_dropped: u64,
    pub channel_blocked: u64,
    pub sink_errors: u64,
    pub avg_latency_us: u64,
    pub latency_distribution: Vec<u64>,
    pub active_workers: i64,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    /// Overall health level (derived from individual sink statuses)
    pub overall_status: SinkStatus,
    pub sinks: HashMap<String, SinkHealth>,
    pub channel_usage: f64,
    pub uptime_seconds: u64,
    pub metrics: MetricsSnapshot,
}

/// Health monitoring metrics collector.
///
/// This struct provides the following accessor methods for reading counter values:
/// - [`logs_written()`](struct.Metrics.html#method.logs_written) - Total logs successfully written
/// - [`logs_dropped()`](struct.Metrics.html#method.logs_dropped) - Total logs dropped
/// - [`channel_blocked()`](struct.Metrics.html#method.channel_blocked) - Total channel blocking events
/// - [`sink_errors()`](struct.Metrics.html#method.sink_errors) - Total sink errors
///
/// # Example
///
/// ```rust
/// use inklog::Metrics;
///
/// let metrics = Metrics::new();
/// metrics.inc_logs_written();
/// assert_eq!(metrics.logs_written(), 1);
/// ```
#[derive(Debug)]
pub struct Metrics {
    pub(crate) logs_written_total: AtomicU64,
    pub(crate) logs_dropped_total: AtomicU64,
    pub(crate) channel_send_blocked_total: AtomicU64,
    pub(crate) sink_errors_total: AtomicU64,
    pub(crate) start_time: Instant,

    // Latency tracking
    pub(crate) total_latency_us: AtomicU64,
    pub(crate) latency_count: AtomicU64,
    pub(crate) latency_histogram: Histogram,

    // Gauges
    pub(crate) active_workers: Gauge,

    // Sink Health
    pub(crate) sink_health: Mutex<HashMap<String, SinkHealth>>,
}

impl Default for Metrics {
    fn default() -> Self {
        // Default buckets: 1ms, 5ms, 10ms, 50ms, 100ms, 500ms, 1s
        let bounds = vec![1000, 5000, 10000, 50000, 100000, 500000, 1000000];
        Self {
            logs_written_total: AtomicU64::new(0),
            logs_dropped_total: AtomicU64::new(0),
            channel_send_blocked_total: AtomicU64::new(0),
            sink_errors_total: AtomicU64::new(0),
            start_time: Instant::now(),
            total_latency_us: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_histogram: Histogram::new(bounds),
            active_workers: Gauge::new(0),
            sink_health: Mutex::new(HashMap::new()),
        }
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Audit helper for internal state access logging.
    /// Only logs when tracing is at debug level or lower.
    #[inline]
    fn audit_access(&self, field: &str) {
        tracing::debug!(event = "internal_state_access", field = field,);
    }

    /// Returns the total number of logs successfully written.
    pub fn logs_written(&self) -> u64 {
        self.logs_written_total.load(Ordering::Relaxed)
    }

    /// Returns the total number of logs dropped.
    pub fn logs_dropped(&self) -> u64 {
        self.logs_dropped_total.load(Ordering::Relaxed)
    }

    /// Returns the total number of times the channel was blocked.
    pub fn channel_blocked(&self) -> u64 {
        self.channel_send_blocked_total.load(Ordering::Relaxed)
    }

    /// Returns the total number of sink errors.
    pub fn sink_errors(&self) -> u64 {
        self.sink_errors_total.load(Ordering::Relaxed)
    }

    /// Returns the number of active workers (with audit logging).
    pub fn active_workers(&self) -> i64 {
        self.audit_access("active_workers");
        self.active_workers.get()
    }

    /// Returns the sink health status map (with audit logging).
    pub fn sink_health(&self) -> std::collections::HashMap<String, SinkHealth> {
        self.audit_access("sink_health");
        match self.sink_health.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => std::collections::HashMap::new(),
        }
    }

    /// Returns the uptime duration.
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn inc_logs_written(&self) {
        self.logs_written_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_logs_dropped(&self) {
        self.logs_dropped_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_channel_blocked(&self) {
        self.channel_send_blocked_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_sink_error(&self) {
        self.sink_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency(&self, duration: Duration) {
        let micros = duration.as_micros() as u64;
        self.total_latency_us.fetch_add(micros, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);
        self.latency_histogram.record(micros);
    }

    /// Updates the health status of a sink component.
    ///
    /// # Arguments
    /// * `name` - The name of the sink
    /// * `healthy` - Whether the sink is healthy
    /// * `error` - Optional error message if the sink is unhealthy
    pub fn update_sink_health(&self, name: &str, healthy: bool, error: Option<String>) {
        // 减少锁持有时间：在锁外准备状态
        let status = if healthy {
            SinkStatus::Healthy
        } else {
            let error_msg = error
                .as_ref()
                .unwrap_or(&"Unknown error".to_string())
                .clone();
            SinkStatus::Unhealthy { error: error_msg }
        };

        let (new_failures, new_error) = if healthy {
            (0, None)
        } else {
            // 需要获取当前失败次数，所以需要先读取
            let current_failures = if let Ok(map) = self.sink_health.lock() {
                map.get(name).map(|h| h.consecutive_failures).unwrap_or(0)
            } else {
                0
            };
            (current_failures + 1, error)
        };

        // 现在快速更新
        if let Ok(mut map) = self.sink_health.lock() {
            let entry = map
                .entry(name.to_string())
                .or_insert_with(SinkHealth::healthy);
            entry.status = status;
            entry.consecutive_failures = new_failures;
            entry.last_error = new_error;
        }
    }

    /// Reports that a sink has started (transitions from NotStarted to Healthy)
    pub fn sink_started(&self, name: &str) {
        if let Ok(mut map) = self.sink_health.lock() {
            let entry = map.entry(name.to_string()).or_insert(SinkHealth::healthy());
            entry.status = SinkStatus::Healthy;
            entry.consecutive_failures = 0;
            entry.last_error = None;
        }
    }

    /// Reports that a sink has degraded but is still operational
    pub fn sink_degraded(&self, name: &str, reason: String) {
        if let Ok(mut map) = self.sink_health.lock() {
            let entry = map.entry(name.to_string()).or_insert(SinkHealth::healthy());
            entry.status = SinkStatus::Degraded {
                reason: reason.clone(),
            };
            entry.last_error = Some(reason);
        }
    }

    pub fn get_status(&self, channel_len: usize, channel_cap: usize) -> HealthStatus {
        let sinks: std::collections::HashMap<String, SinkHealth> = match self.sink_health.lock() {
            Ok(guard) => guard.clone(),
            Err(_e) => {
                eprintln!("Metrics mutex poisoned, using empty data");
                std::collections::HashMap::new()
            }
        };

        // Determine overall status based on sink statuses
        let overall_status = if sinks.is_empty() {
            SinkStatus::NotStarted
        } else {
            let all_healthy = sinks.values().all(|s| s.status.is_fully_healthy());
            let any_unhealthy = sinks
                .values()
                .any(|s| matches!(s.status, SinkStatus::Unhealthy { .. }));
            let any_degraded = sinks
                .values()
                .any(|s| matches!(s.status, SinkStatus::Degraded { .. }));

            if all_healthy {
                SinkStatus::Healthy
            } else if any_unhealthy {
                let errors: Vec<String> = sinks
                    .values()
                    .filter_map(|s| {
                        if let SinkStatus::Unhealthy { error } = &s.status {
                            Some(error.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                SinkStatus::Unhealthy {
                    error: errors.join("; "),
                }
            } else if any_degraded {
                let reasons: Vec<String> = sinks
                    .values()
                    .filter_map(|s| {
                        if let SinkStatus::Degraded { reason } = &s.status {
                            Some(reason.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                SinkStatus::Degraded {
                    reason: reasons.join("; "),
                }
            } else {
                SinkStatus::Healthy
            }
        };

        let count = self.latency_count.load(Ordering::Relaxed);
        let total = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { total / count } else { 0 };

        HealthStatus {
            overall_status,
            sinks,
            channel_usage: if channel_cap > 0 {
                channel_len as f64 / channel_cap as f64
            } else {
                0.0
            },
            uptime_seconds: self.uptime().as_secs(),
            metrics: MetricsSnapshot {
                logs_written: self.logs_written_total.load(Ordering::Relaxed),
                logs_dropped: self.logs_dropped_total.load(Ordering::Relaxed),
                channel_blocked: self.channel_send_blocked_total.load(Ordering::Relaxed),
                sink_errors: self.sink_errors_total.load(Ordering::Relaxed),
                avg_latency_us: avg_latency,
                latency_distribution: self.latency_histogram.snapshot(),
                active_workers: self.active_workers.get(),
            },
        }
    }

    pub fn export_prometheus(&self) -> String {
        let mut s = String::new();
        s.push_str("# HELP inklog_logs_written_total Total logs successfully written\n");
        s.push_str("# TYPE inklog_logs_written_total counter\n");
        s.push_str(&format!(
            "inklog_logs_written_total {}\n",
            self.logs_written_total.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP inklog_logs_dropped_total Total logs dropped\n");
        s.push_str("# TYPE inklog_logs_dropped_total counter\n");
        s.push_str(&format!(
            "inklog_logs_dropped_total {}\n",
            self.logs_dropped_total.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP inklog_channel_blocked_total Total times channel was blocked\n");
        s.push_str("# TYPE inklog_channel_blocked_total counter\n");
        s.push_str(&format!(
            "inklog_channel_blocked_total {}\n",
            self.channel_send_blocked_total.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP inklog_sink_errors_total Total sink errors\n");
        s.push_str("# TYPE inklog_sink_errors_total counter\n");
        s.push_str(&format!(
            "inklog_sink_errors_total {}\n",
            self.sink_errors_total.load(Ordering::Relaxed)
        ));

        s.push_str("# HELP inklog_active_workers Current active worker threads\n");
        s.push_str("# TYPE inklog_active_workers gauge\n");
        s.push_str(&format!(
            "inklog_active_workers {}\n",
            self.active_workers.get()
        ));

        //
        let count = self.latency_count.load(Ordering::Relaxed);
        let total = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { total / count } else { 0 };

        s.push_str("# HELP inklog_avg_latency_us Average log processing latency in microseconds\n");
        s.push_str("# TYPE inklog_avg_latency_us gauge\n");
        s.push_str(&format!("inklog_avg_latency_us {}\n", avg_latency));

        //
        let uptime = self.uptime().as_secs();
        if uptime > 0 {
            s.push_str("# HELP inklog_uptime_seconds Uptime in seconds\n");
            s.push_str("# TYPE inklog_uptime_seconds gauge\n");
            s.push_str(&format!("inklog_uptime_seconds {}\n", uptime));
        }

        //
        s.push_str("# HELP inklog_sink_healthy Sink health status (1=healthy, 0=unhealthy)\n");
        s.push_str("# TYPE inklog_sink_healthy gauge\n");
        if let Ok(health_map) = self.sink_health.lock() {
            for (name, health) in health_map.iter() {
                let value = if health.status.is_operational() { 1 } else { 0 };
                s.push_str(&format!(
                    "inklog_sink_healthy{{sink=\"{}\"}} {}\n",
                    name, value
                ));
            }
        }

        //
        s.push_str("# HELP inklog_latency_bucket Latency histogram bucket\n");
        s.push_str("# TYPE inklog_latency_bucket counter\n");
        let bounds = [1000, 5000, 10000, 50000, 100000, 500000, 1000000];
        let buckets = self.latency_histogram.snapshot();
        for (i, &bound) in bounds.iter().enumerate() {
            if i < buckets.len() {
                s.push_str(&format!(
                    "inklog_latency_bucket{{le=\"{}\"}} {}\n",
                    bound, buckets[i]
                ));
            }
        }
        //
        let total_count: u64 = buckets.iter().sum();
        s.push_str(&format!(
            "inklog_latency_bucket{{le=\"+Inf\"}} {}\n",
            total_count
        ));

        s
    }
}

/// 降级状态追踪
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackState {
    /// 正常运行（未降级）
    Active,
    /// 已降级到备用目标
    Fallback { target: String, reason: String },
    /// 正在恢复（指数退避中）
    Recovering { attempt: u32, delay_ms: u64 },
}

/// Sink 降级配置
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// 是否启用自动降级
    pub enabled: bool,
    /// 初始重试延迟（毫秒）
    pub initial_delay_ms: u64,
    /// 最大重试延迟（毫秒）
    pub max_delay_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 故障阈值（连续失败次数）
    pub failure_threshold: u32,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            max_retries: 10,
            failure_threshold: 3,
        }
    }
}

/// Sink 健康监控器 - 智能降级系统
///
/// 此结构体负责：
/// - 监控各 Sink 的健康状态
/// - 检测故障并触发降级
/// - 使用指数退避实现自动恢复
/// - 记录降级/恢复事件
///
/// ## 降级策略
///
/// | 故障场景 | 降级目标 |
/// |----------|----------|
/// | Database 故障 | FileSink (db_fallback.log) |
/// | FileSink 故障（磁盘满） | ConsoleSink |
/// | S3 不可达 | 本地保留，网络恢复后重试 |
/// | 加密密钥错误 | 明文写入 + 警告日志 |
///
/// ## 使用示例
///
/// ```rust
/// use inklog::metrics::{SinkHealthMonitor, FallbackConfig};
///
/// let config = FallbackConfig::default();
/// let monitor = SinkHealthMonitor::new(config);
///
/// // 检查并可能触发降级
/// let action = monitor.check_and_fallback("database", false, "Connection refused");
/// ```
#[derive(Debug)]
pub struct SinkHealthMonitor {
    /// 降级配置
    config: FallbackConfig,
    /// 各 Sink 的降级状态
    fallback_states: Mutex<HashMap<String, FallbackState>>,
    /// 重试计数器（用于指数退避）
    retry_counters: Mutex<HashMap<String, u32>>,
    /// 降级事件日志
    fallback_events: Mutex<Vec<FallbackEvent>>,
}

/// 降级事件记录
#[derive(Debug, Clone)]
pub struct FallbackEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sink_name: String,
    pub from_state: FallbackState,
    pub to_state: FallbackState,
    pub reason: String,
}

impl Default for SinkHealthMonitor {
    fn default() -> Self {
        Self::new(FallbackConfig::default())
    }
}

impl SinkHealthMonitor {
    /// 创建新的 Sink 健康监控器
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            config,
            fallback_states: Mutex::new(HashMap::new()),
            retry_counters: Mutex::new(HashMap::new()),
            fallback_events: Mutex::new(Vec::new()),
        }
    }

    /// 使用默认配置创建监控器
    pub fn with_defaults() -> Self {
        Self::new(FallbackConfig::default())
    }

    /// 检查 Sink 状态并可能触发降级
    ///
    /// # Arguments
    ///
    /// * `sink_name` - Sink 名称
    /// * `is_healthy` - Sink 当前是否健康
    /// * `error` - 错误信息（如果故障）
    ///
    /// # Returns
    ///
    /// 建议的降级操作
    pub fn check_and_fallback(
        &self,
        sink_name: &str,
        is_healthy: bool,
        error: Option<&str>,
    ) -> FallbackAction {
        let mut states = self
            .fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut retries = self
            .retry_counters
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let current_state = states
            .get(sink_name)
            .cloned()
            .unwrap_or(FallbackState::Active);

        if is_healthy {
            // Sink 恢复健康，检查是否可以恢复
            self.handle_recovery(sink_name, &current_state, &mut states, &mut retries)
        } else {
            // Sink 故障，触发降级
            let error_msg = error.unwrap_or("Unknown error").to_string();
            self.handle_failure(
                sink_name,
                &error_msg,
                &current_state,
                &mut states,
                &mut retries,
            )
        }
    }

    /// 处理恢复场景
    fn handle_recovery(
        &self,
        sink_name: &str,
        current_state: &FallbackState,
        states: &mut HashMap<String, FallbackState>,
        retries: &mut HashMap<String, u32>,
    ) -> FallbackAction {
        match current_state {
            FallbackState::Active => {
                // 已经在正常运行状态
                FallbackAction::None
            }
            FallbackState::Fallback { target, reason: _ } => {
                // 之前降级到备用目标，现在可以尝试恢复
                tracing::info!(
                    event = "sink_recovering",
                    sink = sink_name,
                    fallback_target = target,
                    "Sink {} 正在从 {} 恢复",
                    sink_name,
                    target
                );

                let attempt = retries.get(sink_name).cloned().unwrap_or(0) + 1;
                retries.insert(sink_name.to_string(), attempt);

                // 计算指数退避延迟
                let delay_ms = self
                    .config
                    .initial_delay_ms
                    .saturating_mul(2_u64.pow(attempt.min(10)))
                    .min(self.config.max_delay_ms);

                states.insert(
                    sink_name.to_string(),
                    FallbackState::Recovering { attempt, delay_ms },
                );

                self.log_event(
                    sink_name,
                    current_state.clone(),
                    states.get(sink_name).unwrap().clone(),
                    format!("尝试恢复，延迟 {}ms", delay_ms),
                );

                FallbackAction::AttemptRecovery {
                    sink_name: sink_name.to_string(),
                    attempt,
                    delay_ms,
                }
            }
            FallbackState::Recovering {
                attempt: _,
                delay_ms,
            } => {
                // 正在恢复中，继续等待
                FallbackAction::Wait {
                    sink_name: sink_name.to_string(),
                    remaining_ms: *delay_ms,
                }
            }
        }
    }

    /// 处理故障场景
    fn handle_failure(
        &self,
        sink_name: &str,
        error: &str,
        current_state: &FallbackState,
        states: &mut HashMap<String, FallbackState>,
        retries: &mut HashMap<String, u32>,
    ) -> FallbackAction {
        let failure_count = retries.get(sink_name).cloned().unwrap_or(0) + 1;
        retries.insert(sink_name.to_string(), failure_count);

        // 确定降级目标
        let (fallback_target, action) = self.determine_fallback_target(sink_name, error);

        // 如果降级配置未启用，只记录警告
        if !self.config.enabled {
            tracing::warn!(
                event = "sink_failed_disabled_fallback",
                sink = sink_name,
                error = error,
                "Sink {} 故障但自动降级已禁用",
                sink_name
            );
            return FallbackAction::None;
        }

        // 如果达到故障阈值，触发降级
        if failure_count >= self.config.failure_threshold {
            let new_state = FallbackState::Fallback {
                target: fallback_target.clone(),
                reason: error.to_string(),
            };

            tracing::warn!(
                event = "sink_fallback_triggered",
                sink = sink_name,
                fallback_target = fallback_target,
                error = error,
                failure_count = failure_count,
                "Sink {} 降级到 {}，原因: {}",
                sink_name,
                fallback_target,
                error
            );

            states.insert(sink_name.to_string(), new_state.clone());
            self.log_event(
                sink_name,
                current_state.clone(),
                new_state,
                error.to_string(),
            );

            // 重置重试计数器
            retries.insert(sink_name.to_string(), 0);

            action
        } else {
            // 还未达到阈值，记录警告
            tracing::warn!(
                event = "sink_failure_warning",
                sink = sink_name,
                error = error,
                failure_count = failure_count,
                threshold = self.config.failure_threshold,
                "Sink {} 连续第 {} 次故障",
                sink_name,
                failure_count
            );

            FallbackAction::Retry {
                sink_name: sink_name.to_string(),
                attempt: failure_count,
                error: error.to_string(),
            }
        }
    }

    /// 确定降级目标
    fn determine_fallback_target(&self, sink_name: &str, error: &str) -> (String, FallbackAction) {
        match sink_name {
            "database" => {
                // Database 故障 -> 降级到 FileSink
                (
                    "file".to_string(),
                    FallbackAction::Fallback {
                        sink_name: sink_name.to_string(),
                        target: "file".to_string(),
                        reason: format!("Database 故障: {}", error),
                    },
                )
            }
            "file" => {
                // FileSink 故障（磁盘满） -> 降级到 ConsoleSink
                let lower_error = error.to_lowercase();
                if lower_error.contains("disk")
                    || lower_error.contains("space")
                    || lower_error.contains("full")
                {
                    (
                        "console".to_string(),
                        FallbackAction::Fallback {
                            sink_name: sink_name.to_string(),
                            target: "console".to_string(),
                            reason: format!("磁盘空间不足: {}", error),
                        },
                    )
                } else {
                    // 其他文件错误也降级到 console
                    (
                        "console".to_string(),
                        FallbackAction::Fallback {
                            sink_name: sink_name.to_string(),
                            target: "console".to_string(),
                            reason: format!("FileSink 故障: {}", error),
                        },
                    )
                }
            }
            "s3" | "s3_archive" => {
                // S3 不可达 -> 本地保留，网络恢复后重试
                (
                    "local".to_string(),
                    FallbackAction::LocalQueue {
                        sink_name: sink_name.to_string(),
                        reason: format!("S3 不可达: {}，本地保留待重试", error),
                    },
                )
            }
            _ => {
                // 未知 Sink，默认降级到 console
                (
                    "console".to_string(),
                    FallbackAction::Fallback {
                        sink_name: sink_name.to_string(),
                        target: "console".to_string(),
                        reason: format!("未知故障: {}", error),
                    },
                )
            }
        }
    }

    /// 处理加密密钥错误（特殊场景）
    ///
    /// 加密密钥错误时，降级为明文写入 + 警告日志
    pub fn handle_encryption_error(&self, sink_name: &str, error: &str) -> FallbackAction {
        tracing::warn!(
            event = "encryption_error_fallback",
            sink = sink_name,
            error = error,
            "加密密钥错误，降级为明文写入"
        );

        let mut states = self
            .fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let current_state = states
            .get(sink_name)
            .cloned()
            .unwrap_or(FallbackState::Active);

        let new_state = FallbackState::Fallback {
            target: "plaintext".to_string(),
            reason: format!("加密密钥错误: {}", error),
        };

        states.insert(sink_name.to_string(), new_state.clone());
        self.log_event(sink_name, current_state, new_state, error.to_string());

        FallbackAction::Fallback {
            sink_name: sink_name.to_string(),
            target: "plaintext".to_string(),
            reason: format!("明文写入（加密错误）: {}", error),
        }
    }

    /// 确认恢复成功
    pub fn confirm_recovery(&self, sink_name: &str) {
        let mut states = self
            .fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut retries = self
            .retry_counters
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let current_state = states
            .get(sink_name)
            .cloned()
            .unwrap_or(FallbackState::Active);

        if !matches!(current_state, FallbackState::Active) {
            tracing::info!(
                event = "sink_recovery_confirmed",
                sink = sink_name,
                "Sink {} 恢复成功，已切回正常模式",
                sink_name
            );

            states.insert(sink_name.to_string(), FallbackState::Active);
            retries.remove(sink_name);
            self.log_event(
                sink_name,
                current_state,
                FallbackState::Active,
                "恢复成功".to_string(),
            );
        }
    }

    /// 获取当前降级状态
    pub fn get_fallback_state(&self, sink_name: &str) -> FallbackState {
        self.fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(sink_name)
            .cloned()
            .unwrap_or(FallbackState::Active)
    }

    /// 检查是否有任何 Sink 处于降级状态
    pub fn is_any_in_fallback(&self) -> bool {
        self.fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .any(|state| matches!(state, FallbackState::Fallback { .. }))
    }

    /// 获取所有降级事件
    pub fn get_fallback_events(&self, limit: usize) -> Vec<FallbackEvent> {
        let events = self
            .fallback_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        events.iter().rev().take(limit).cloned().collect()
    }

    /// 获取降级统计
    pub fn get_fallback_stats(&self) -> FallbackStats {
        let states = self
            .fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let events = self
            .fallback_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let active_fallbacks = states
            .values()
            .filter(|s| matches!(s, FallbackState::Fallback { .. }))
            .count();
        let recovering = states
            .values()
            .filter(|s| matches!(s, FallbackState::Recovering { .. }))
            .count();
        let fallback_events_count = events.len();

        FallbackStats {
            active_fallbacks,
            recovering,
            total_fallback_events: fallback_events_count,
        }
    }

    /// 记录降级事件
    fn log_event(&self, sink_name: &str, from: FallbackState, to: FallbackState, reason: String) {
        let event = FallbackEvent {
            timestamp: chrono::Utc::now(),
            sink_name: sink_name.to_string(),
            from_state: from,
            to_state: to,
            reason,
        };

        let mut events = self
            .fallback_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        events.push(event);

        // 保留最近 100 个事件
        if events.len() > 100 {
            events.remove(0);
        }
    }

    /// 重置监控器状态
    pub fn reset(&self) {
        let mut states = self
            .fallback_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut retries = self
            .retry_counters
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let mut events = self
            .fallback_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        states.clear();
        retries.clear();
        events.clear();
    }
}

/// 降级操作建议
#[derive(Debug, Clone)]
pub enum FallbackAction {
    /// 无操作
    None,
    /// 重试当前操作
    Retry {
        sink_name: String,
        attempt: u32,
        error: String,
    },
    /// 降级到备用目标
    Fallback {
        sink_name: String,
        target: String,
        reason: String,
    },
    /// 尝试恢复
    AttemptRecovery {
        sink_name: String,
        attempt: u32,
        delay_ms: u64,
    },
    /// 等待恢复
    Wait {
        sink_name: String,
        remaining_ms: u64,
    },
    /// 本地队列等待重试
    LocalQueue { sink_name: String, reason: String },
}

impl FallbackAction {
    /// 检查是否需要执行操作
    pub fn requires_action(&self) -> bool {
        !matches!(self, FallbackAction::None)
    }

    /// 获取相关 Sink 名称
    pub fn sink_name(&self) -> Option<&str> {
        match self {
            FallbackAction::None => None,
            FallbackAction::Retry { sink_name, .. } => Some(sink_name),
            FallbackAction::Fallback { sink_name, .. } => Some(sink_name),
            FallbackAction::AttemptRecovery { sink_name, .. } => Some(sink_name),
            FallbackAction::Wait { sink_name, .. } => Some(sink_name),
            FallbackAction::LocalQueue { sink_name, .. } => Some(sink_name),
        }
    }
}

/// 降级统计信息
#[derive(Debug, Clone, Default)]
pub struct FallbackStats {
    pub active_fallbacks: usize,
    pub recovering: usize,
    pub total_fallback_events: usize,
}

#[cfg(test)]
mod sink_health_monitor_tests {
    use super::*;

    #[test]
    fn test_fallback_state_operations() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 初始状态应该是 Active
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
        assert!(!monitor.is_any_in_fallback());

        // 模拟故障
        let action = monitor.check_and_fallback("database", false, Some("Connection refused"));
        assert!(action.requires_action());
        assert!(matches!(action, FallbackAction::Retry { .. }));

        // 多次故障后应该触发降级
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("Connection refused"));
        }

        // 现在应该处于降级状态
        let state = monitor.get_fallback_state("database");
        assert!(matches!(state, FallbackState::Fallback { target, .. } if target == "file"));

        // 恢复健康后应该尝试恢复
        let action = monitor.check_and_fallback("database", true, None);
        assert!(matches!(action, FallbackAction::AttemptRecovery { .. }));

        // 确认恢复
        monitor.confirm_recovery("database");
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
    }

    #[test]
    fn test_file_sink_disk_full_fallback() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 触发磁盘满故障
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("file", false, Some("Disk is full"));
        }

        let state = monitor.get_fallback_state("file");
        assert!(matches!(
            state,
            FallbackState::Fallback { target, .. } if target == "console"
        ));
    }

    #[test]
    fn test_encryption_error_fallback() {
        let monitor = SinkHealthMonitor::with_defaults();

        let action = monitor.handle_encryption_error("file", "Invalid key");
        assert!(matches!(action, FallbackAction::Fallback { target, .. } if target == "plaintext"));
    }

    #[test]
    fn test_fallback_stats() {
        let monitor = SinkHealthMonitor::with_defaults();

        let stats = monitor.get_fallback_stats();
        assert_eq!(stats.active_fallbacks, 0);
        assert_eq!(stats.recovering, 0);
    }

    #[test]
    fn test_fallback_events() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 触发一次降级
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("Error"));
        }

        let events = monitor.get_fallback_events(10);
        assert!(!events.is_empty());
        assert_eq!(events[0].sink_name, "database");
    }

    #[test]
    fn test_disabled_fallback() {
        let config = FallbackConfig {
            enabled: false,
            ..Default::default()
        };
        let monitor = SinkHealthMonitor::new(config);

        // 禁用降级时，即使故障也不应该降级
        let action = monitor.check_and_fallback("database", false, Some("Error"));
        assert!(!action.requires_action());
    }

    #[test]
    fn test_reset() {
        let monitor = SinkHealthMonitor::with_defaults();

        // 触发降级
        for _ in 0..3 {
            let _ = monitor.check_and_fallback("database", false, Some("Error"));
        }

        assert!(monitor.is_any_in_fallback());

        // 重置
        monitor.reset();

        assert!(!monitor.is_any_in_fallback());
        assert_eq!(
            monitor.get_fallback_state("database"),
            FallbackState::Active
        );
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;

    #[test]
    fn test_gauge_new() {
        let gauge = Gauge::new(100);
        assert_eq!(gauge.get(), 100);
    }

    #[test]
    fn test_gauge_set() {
        let gauge = Gauge::new(0);
        gauge.set(42);
        assert_eq!(gauge.get(), 42);
    }

    #[test]
    fn test_gauge_inc() {
        let gauge = Gauge::new(0);
        gauge.inc();
        gauge.inc();
        assert_eq!(gauge.get(), 2);
    }

    #[test]
    fn test_gauge_dec() {
        let gauge = Gauge::new(10);
        gauge.dec();
        gauge.dec();
        assert_eq!(gauge.get(), 8);
    }

    #[test]
    fn test_histogram_new() {
        let histogram = Histogram::new(vec![100, 500, 1000]);
        assert_eq!(histogram.buckets.len(), 4);
    }

    #[test]
    fn test_histogram_record() {
        let histogram = Histogram::new(vec![100, 500, 1000]);
        histogram.record(50);
        histogram.record(200);
        histogram.record(600);
        histogram.record(1500);

        let snapshot = histogram.snapshot();
        // First bucket should have 1 (50 < 100)
        assert_eq!(snapshot[0], 1);
        // Second bucket should have 1 (100 <= 200 < 500)
        assert_eq!(snapshot[1], 1);
        // Third bucket should have 1 (500 <= 600 < 1000)
        assert_eq!(snapshot[2], 1);
        // Last bucket should have 1 (1500 >= 1000)
        assert_eq!(snapshot[3], 1);
    }

    #[test]
    fn test_sink_health_healthy() {
        let health = SinkHealth::healthy();
        assert!(matches!(health.status, SinkStatus::Healthy));
        assert!(health.last_error.is_none());
        assert_eq!(health.consecutive_failures, 0);
    }

    #[test]
    fn test_sink_health_unhealthy() {
        let health = SinkHealth::unhealthy("Connection refused".to_string());
        assert!(matches!(health.status, SinkStatus::Unhealthy { .. }));
        assert!(health.last_error.is_some());
        assert_eq!(health.consecutive_failures, 1);
    }

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        // Verify metrics are initialized correctly
        assert_eq!(metrics.logs_written(), 0);
        assert_eq!(metrics.logs_dropped(), 0);
        assert_eq!(metrics.sink_errors(), 0);
    }

    #[test]
    fn test_metrics_record_log_written() {
        let metrics = Metrics::new();
        metrics.inc_logs_written();
        assert_eq!(metrics.logs_written(), 1);
        metrics.inc_logs_written();
        assert_eq!(metrics.logs_written(), 2);
    }

    #[test]
    fn test_metrics_record_log_dropped() {
        let metrics = Metrics::new();
        metrics.inc_logs_dropped();
        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_metrics_record_sink_error() {
        let metrics = Metrics::new();
        metrics.inc_sink_error();
        assert_eq!(metrics.sink_errors(), 1);
    }

    #[test]
    fn test_metrics_record_channel_blocked() {
        let metrics = Metrics::new();
        metrics.inc_channel_blocked();
        assert_eq!(metrics.channel_blocked(), 1);
    }

    #[test]
    fn test_metrics_record_latency() {
        let metrics = Metrics::new();
        metrics.record_latency(Duration::from_micros(100));
        metrics.record_latency(Duration::from_micros(200));
        metrics.record_latency(Duration::from_micros(300));
        // Should have recorded latencies
        let latency = metrics.total_latency_us.load(Ordering::Relaxed);
        assert!(latency >= 100 + 200 + 300);
    }

    #[test]
    fn test_metrics_update_sink_health() {
        let metrics = Metrics::new();
        metrics.update_sink_health("file", true, None);
        let health = metrics.sink_health();
        assert!(health.contains_key("file"));
    }

    #[test]
    fn test_metrics_active_workers() {
        let metrics = Metrics::new();
        metrics.active_workers.set(4);
        assert_eq!(metrics.active_workers(), 4);
    }

    #[test]
    fn test_metrics_logs_written() {
        let metrics = Metrics::new();
        metrics.inc_logs_written();
        metrics.inc_logs_written();
        assert_eq!(metrics.logs_written(), 2);
    }

    #[test]
    fn test_metrics_logs_dropped() {
        let metrics = Metrics::new();
        metrics.inc_logs_dropped();
        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_metrics_channel_blocked() {
        let metrics = Metrics::new();
        metrics.inc_channel_blocked();
        assert_eq!(metrics.channel_blocked(), 1);
    }

    #[test]
    fn test_metrics_sink_errors() {
        let metrics = Metrics::new();
        metrics.inc_sink_error();
        assert_eq!(metrics.sink_errors(), 1);
    }

    #[test]
    fn test_metrics_uptime() {
        let metrics = Metrics::new();
        let uptime = metrics.uptime();
        assert!(uptime.as_nanos() >= 0);
    }
}
