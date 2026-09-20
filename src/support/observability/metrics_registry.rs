// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 通用业务指标 registry（Prometheus 文本格式 0.0.4 兼容导出）。
//!
//! 与 [`super::metrics`]（日志管线自身指标）的区别：本模块面向**业务侧**
//! 任意命名的 counter/gauge/histogram——业务服务把自有业务指标注册进
//! registry，经 [`MetricsRegistry::export_prometheus`] 渲染为标准
//! Prometheus 文本格式，挂到业务自己的 `/metrics` 端点。
//!
//! # 设计
//!
//! - **标签维度**：`*Vec` 系列（[`CounterVec`] 等）按 label_names 声明维度，
//!   `child(&[...])` 取时间序列；子序列首次访问时创建（Prometheus 惯例）
//! - **并发**：registry 级 `Mutex<BTreeMap>`（注册低频），子序列值用原子量
//!   （写高频，无锁）
//! - **确定性导出**：BTreeMap + 序列遍历按标签值排序，同输入同输出
//! - **重注册校验**：同名指标重复注册时校验 help/标签/buckets 一致，
//!   不一致 panic（编程期错误显性失败，Rule 12）
//!
//! # 示例
//!
//! ```
//! use inklog::support::observability::metrics_registry::MetricsRegistry;
//!
//! let registry = MetricsRegistry::new();
//! let requests = registry.register_counter_vec(
//!     "business_requests_total",
//!     "Business requests by endpoint",
//!     &["endpoint"],
//! );
//! requests.child(&["/api/v1/login"]).inc();
//! let text = registry.export_prometheus();
//! assert!(text.contains("business_requests_total{endpoint=\"/api/v1/login\"} 1"));
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 单条时间序列的标签取值（与 label_names 顺序对齐）。
type LabelValues = Vec<String>;

/// 指标类型（注册一致性校验用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

/// histogram 子序列（bucket 累积计数 + sum/count）。
#[derive(Debug)]
struct HistogramSeries {
    /// 各 bucket 累积计数（与 buckets 等长；Prometheus 语义为 ≤ 上界的累积值）。
    bucket_counts: Vec<AtomicU64>,
    /// f64 的位模式存储（AtomicF64 无标准实现）。
    sum_bits: AtomicU64,
    count: AtomicU64,
}

/// registry 内部的指标族（同名 metric 的全部时间序列）。
#[derive(Debug)]
enum MetricFamily {
    Counter {
        help: String,
        label_names: Vec<String>,
        series: BTreeMap<LabelValues, Arc<AtomicU64>>,
    },
    Gauge {
        help: String,
        label_names: Vec<String>,
        /// i64 位模式存储（gauge 可 set 负值）。
        series: BTreeMap<LabelValues, Arc<AtomicU64>>,
    },
    Histogram {
        help: String,
        label_names: Vec<String>,
        /// 显式 bucket 上界（不含 +Inf；已按注册入参排序去重）。
        buckets: Vec<f64>,
        series: BTreeMap<LabelValues, Arc<HistogramSeries>>,
    },
}

impl MetricFamily {
    fn help(&self) -> &str {
        match self {
            MetricFamily::Counter { help, .. }
            | MetricFamily::Gauge { help, .. }
            | MetricFamily::Histogram { help, .. } => help,
        }
    }

    fn label_names(&self) -> &[String] {
        match self {
            MetricFamily::Counter { label_names, .. }
            | MetricFamily::Gauge { label_names, .. }
            | MetricFamily::Histogram { label_names, .. } => label_names,
        }
    }
}

/// registry 内部状态（Arc 共享：`*Vec` 句柄持有它，registry 克隆零拷贝）。
#[derive(Debug, Default)]
struct RegistryInner {
    families: Mutex<BTreeMap<String, MetricFamily>>,
}

impl RegistryInner {
    /// 注册（或校验已注册）指标族；参数不一致时 panic（编程期错误显性失败）。
    fn register(
        &self,
        name: &str,
        help: &str,
        label_names: Vec<String>,
        kind: MetricKind,
        buckets: Option<Vec<f64>>,
    ) {
        let mut families = self.families.lock().unwrap();
        match families.get(name) {
            Some(existing) => {
                assert_eq!(existing.help(), help, "metrics `{name}` help 不一致");
                assert_eq!(
                    existing.label_names(),
                    label_names.as_slice(),
                    "metrics `{name}` 标签不一致"
                );
                let existing_kind = match existing {
                    MetricFamily::Counter { .. } => MetricKind::Counter,
                    MetricFamily::Gauge { .. } => MetricKind::Gauge,
                    MetricFamily::Histogram { .. } => MetricKind::Histogram,
                };
                assert_eq!(existing_kind, kind, "metrics `{name}` 类型冲突");
                if let (MetricKind::Histogram, MetricFamily::Histogram { buckets: b0, .. }) =
                    (kind, existing)
                {
                    assert_eq!(
                        b0,
                        buckets.as_ref().expect("histogram 必带 buckets"),
                        "metrics `{name}` buckets 不一致"
                    );
                }
            }
            None => {
                let family = match kind {
                    MetricKind::Counter => MetricFamily::Counter {
                        help: help.to_string(),
                        label_names,
                        series: BTreeMap::new(),
                    },
                    MetricKind::Gauge => MetricFamily::Gauge {
                        help: help.to_string(),
                        label_names,
                        series: BTreeMap::new(),
                    },
                    MetricKind::Histogram => MetricFamily::Histogram {
                        help: help.to_string(),
                        label_names,
                        buckets: buckets.expect("histogram 必带 buckets"),
                        series: BTreeMap::new(),
                    },
                };
                families.insert(name.to_string(), family);
            }
        }
    }

    /// 取（或创建）counter 子序列。
    fn counter_series(&self, name: &str, values: &[String]) -> Arc<AtomicU64> {
        let mut families = self.families.lock().unwrap();
        let Some(MetricFamily::Counter { series, .. }) = families.get_mut(name) else {
            unreachable!("counter 已注册且类型校验过");
        };
        series
            .entry(values.to_vec())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone()
    }

    /// 取（或创建）gauge 子序列。
    fn gauge_series(&self, name: &str, values: &[String]) -> Arc<AtomicU64> {
        let mut families = self.families.lock().unwrap();
        let Some(MetricFamily::Gauge { series, .. }) = families.get_mut(name) else {
            unreachable!("gauge 已注册且类型校验过");
        };
        series
            .entry(values.to_vec())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone()
    }

    /// 取（或创建）histogram 子序列。
    fn histogram_series(&self, name: &str, values: &[String]) -> Arc<HistogramSeries> {
        let mut families = self.families.lock().unwrap();
        let Some(MetricFamily::Histogram {
            buckets, series, ..
        }) = families.get_mut(name)
        else {
            unreachable!("histogram 已注册且类型校验过");
        };
        let buckets = buckets.clone();
        series
            .entry(values.to_vec())
            .or_insert_with(|| {
                Arc::new(HistogramSeries {
                    bucket_counts: buckets
                        .iter()
                        .map(|_| AtomicU64::new(0))
                        .collect::<Vec<_>>(),
                    sum_bits: AtomicU64::new(0),
                    count: AtomicU64::new(0),
                })
            })
            .clone()
    }
}

/// 通用业务指标 registry。
///
/// 克隆廉价（内部 Arc 共享），`*Vec` 句柄可在结构体间自由传递。
#[derive(Debug, Clone, Default)]
pub struct MetricsRegistry {
    inner: Arc<RegistryInner>,
}

impl MetricsRegistry {
    /// 创建空 registry。
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册（或取回）一个 CounterVec。
    pub fn register_counter_vec(&self, name: &str, help: &str, label_names: &[&str]) -> CounterVec {
        self.inner.register(
            name,
            help,
            label_names.iter().map(|s| s.to_string()).collect(),
            MetricKind::Counter,
            None,
        );
        CounterVec {
            name: name.to_string(),
            inner: Arc::clone(&self.inner),
        }
    }

    /// 注册（或取回）一个 GaugeVec。
    pub fn register_gauge_vec(&self, name: &str, help: &str, label_names: &[&str]) -> GaugeVec {
        self.inner.register(
            name,
            help,
            label_names.iter().map(|s| s.to_string()).collect(),
            MetricKind::Gauge,
            None,
        );
        GaugeVec {
            name: name.to_string(),
            inner: Arc::clone(&self.inner),
        }
    }

    /// 注册（或取回）一个 HistogramVec；`buckets` 为显式上界（自动排序，不含 +Inf）。
    pub fn register_histogram_vec(
        &self,
        name: &str,
        help: &str,
        label_names: &[&str],
        buckets: &[f64],
    ) -> HistogramVec {
        let mut buckets = buckets.to_vec();
        buckets.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        buckets.dedup();
        self.inner.register(
            name,
            help,
            label_names.iter().map(|s| s.to_string()).collect(),
            MetricKind::Histogram,
            Some(buckets.clone()),
        );
        HistogramVec {
            name: name.to_string(),
            buckets,
            inner: Arc::clone(&self.inner),
        }
    }

    /// 渲染为 Prometheus 文本格式（0.0.4）。
    pub fn export_prometheus(&self) -> String {
        let families = self.inner.families.lock().unwrap();
        let mut out = String::new();
        for (name, family) in families.iter() {
            let _ = writeln!(out, "# HELP {name} {}", family.help());
            let type_name = match family {
                MetricFamily::Counter { .. } => "counter",
                MetricFamily::Gauge { .. } => "gauge",
                MetricFamily::Histogram { .. } => "histogram",
            };
            let _ = writeln!(out, "# TYPE {name} {type_name}");
            let label_names = family.label_names();
            match family {
                MetricFamily::Counter { series, .. } => {
                    for (values, value) in series.iter() {
                        let _ = writeln!(
                            out,
                            "{}{} {}",
                            name,
                            render_labels(label_names, values),
                            value.load(Ordering::Relaxed)
                        );
                    }
                }
                MetricFamily::Gauge { series, .. } => {
                    for (values, value) in series.iter() {
                        // gauge 以 i64 位模式存储（可负）
                        let bits = value.load(Ordering::Relaxed) as i64;
                        let _ = writeln!(
                            out,
                            "{}{} {}",
                            name,
                            render_labels(label_names, values),
                            bits
                        );
                    }
                }
                MetricFamily::Histogram {
                    buckets, series, ..
                } => {
                    for (values, hist) in series.iter() {
                        let labels = render_labels(label_names, values);
                        // buckets 行：{dim="v",le="bound"}（Prometheus 要求 le 与维度
                        // 标签并列，因此不能复用 render_labels 的整体括号）
                        let mut cumulative = 0u64;
                        // bucket_counts 为非累积计数，导出时渲染为累积值（Prometheus 语义）
                        for (bound, bucket) in buckets.iter().zip(hist.bucket_counts.iter()) {
                            cumulative += bucket.load(Ordering::Relaxed);
                            let _ = writeln!(
                                out,
                                "{}_bucket{} {}",
                                name,
                                render_le_labels(label_names, values, *bound),
                                cumulative
                            );
                        }
                        let _ = writeln!(
                            out,
                            "{}_bucket{} {}",
                            name,
                            render_le_labels(label_names, values, f64::INFINITY),
                            hist.count.load(Ordering::Relaxed)
                        );
                        let sum = f64::from_bits(hist.sum_bits.load(Ordering::Relaxed));
                        let _ = writeln!(out, "{}_sum{} {sum}", name, labels);
                        let _ = writeln!(
                            out,
                            "{}_count{} {}",
                            name,
                            labels,
                            hist.count.load(Ordering::Relaxed)
                        );
                    }
                }
            }
        }
        out
    }
}

/// CounterVec：单调递增计数器（按标签维度）。
#[derive(Debug, Clone)]
pub struct CounterVec {
    name: String,
    inner: Arc<RegistryInner>,
}

impl CounterVec {
    /// 取（或创建）一个子序列计数器。
    pub fn child(&self, label_values: &[&str]) -> Counter {
        let values: Vec<String> = label_values.iter().map(|s| s.to_string()).collect();
        Counter {
            value: self.inner.counter_series(&self.name, &values),
        }
    }
}

/// 单序列计数器句柄。
#[derive(Debug)]
pub struct Counter {
    value: Arc<AtomicU64>,
}

impl Counter {
    /// 计数 +1。
    pub fn inc(&self) {
        self.inc_by(1);
    }

    /// 计数 +n。
    pub fn inc_by(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// 当前值（测试/快照用）。
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

/// GaugeVec：可增可减可置值的仪表。
#[derive(Debug, Clone)]
pub struct GaugeVec {
    name: String,
    inner: Arc<RegistryInner>,
}

impl GaugeVec {
    /// 取（或创建）一个子序列仪表。
    pub fn child(&self, label_values: &[&str]) -> Gauge {
        let values: Vec<String> = label_values.iter().map(|s| s.to_string()).collect();
        Gauge {
            value: self.inner.gauge_series(&self.name, &values),
        }
    }
}

/// 单序列仪表句柄（i64 位模式存储，支持负值）。
#[derive(Debug)]
pub struct Gauge {
    value: Arc<AtomicU64>,
}

impl Gauge {
    /// 设置值。
    pub fn set_i64(&self, v: i64) {
        self.value.store(v as u64, Ordering::Relaxed);
    }

    /// 当前值。
    pub fn get_i64(&self) -> i64 {
        self.value.load(Ordering::Relaxed) as i64
    }
}

/// HistogramVec：观测值分布。
#[derive(Debug, Clone)]
pub struct HistogramVec {
    name: String,
    buckets: Vec<f64>,
    inner: Arc<RegistryInner>,
}

impl HistogramVec {
    /// 取（或创建）一个子序列直方图并观测 `value`。
    pub fn observe(&self, label_values: &[&str], value: f64) {
        let values: Vec<String> = label_values.iter().map(|s| s.to_string()).collect();
        let series = self.inner.histogram_series(&self.name, &values);
        // 值只计入命中的第一个 bucket（各 bucket 为非累积计数；
        // 超过最大显式上界的值仅体现在 count/+Inf，导出时补齐）
        for (i, bound) in self.buckets.iter().enumerate() {
            if value <= *bound {
                series.bucket_counts[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
        series.sum_bits.store(
            (f64::from_bits(series.sum_bits.load(Ordering::Relaxed)) + value).to_bits(),
            Ordering::Relaxed,
        );
        series.count.fetch_add(1, Ordering::Relaxed);
    }
}

/// 渲染 `{a="1",b="2"}`；无标签时返回空串。
fn render_labels(label_names: &[String], values: &[String]) -> String {
    if label_names.is_empty() {
        return String::new();
    }
    let pairs: Vec<String> = label_names
        .iter()
        .zip(values.iter())
        .map(|(n, v)| format!("{n}=\"{}\"", escape_label(v)))
        .collect();
    format!("{{{}}}", pairs.join(","))
}

/// 直方图 bucket 行标签：维度标签与 `le` 并列（`{dim="v",le="1"}` / `{le="1"}`）。
fn render_le_labels(label_names: &[String], values: &[String], bound: f64) -> String {
    let le = format_le(&bound);
    if label_names.is_empty() {
        return format!("{{le=\"{le}\"}}");
    }
    let mut pairs: Vec<String> = label_names
        .iter()
        .zip(values.iter())
        .map(|(n, v)| format!("{n}=\"{}\"", escape_label(v)))
        .collect();
    pairs.push(format!("le=\"{le}\""));
    format!("{{{}}}", pairs.join(","))
}

/// f64 上界渲染（整数不带小数点，+Inf 特判，与 prometheus client 惯例对齐）。
fn format_le(bound: &f64) -> String {
    if bound.is_infinite() && *bound > 0.0 {
        "+Inf".to_string()
    } else if bound.fract() == 0.0 && bound.abs() < 1e15 {
        format!("{}", *bound as i64)
    } else {
        format!("{bound}")
    }
}

/// 标签值转义（`\` `"` `\n`）。
fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// counter：inc/inc_by 与导出格式（含标签）。
    #[test]
    fn counter_vec_inc_and_export() {
        let registry = MetricsRegistry::new();
        let requests = registry.register_counter_vec(
            "business_requests_total",
            "Business requests",
            &["endpoint", "code"],
        );
        requests.child(&["/login", "200"]).inc();
        requests.child(&["/login", "200"]).inc();
        requests.child(&["/login", "500"]).inc_by(3);

        let text = registry.export_prometheus();
        assert!(text.contains("# HELP business_requests_total Business requests"));
        assert!(text.contains("# TYPE business_requests_total counter"));
        assert!(text.contains("business_requests_total{endpoint=\"/login\",code=\"200\"} 2"));
        assert!(text.contains("business_requests_total{endpoint=\"/login\",code=\"500\"} 3"));
    }

    /// gauge：set/负值导出。
    #[test]
    fn gauge_vec_set_and_negative() {
        let registry = MetricsRegistry::new();
        let backlog = registry.register_gauge_vec("business_backlog", "Backlog size", &["team"]);
        backlog.child(&["alpha"]).set_i64(7);
        backlog.child(&["alpha"]).set_i64(-2);
        let text = registry.export_prometheus();
        assert!(text.contains("business_backlog{team=\"alpha\"} -2"));
    }

    /// histogram：bucket 累积、+Inf 行、sum/count。
    #[test]
    fn histogram_vec_observe_and_export() {
        let registry = MetricsRegistry::new();
        let latency = registry.register_histogram_vec(
            "business_latency_seconds",
            "Latency",
            &[],
            &[0.1, 1.0, 10.0],
        );
        latency.observe(&[], 0.05);
        latency.observe(&[], 0.5);
        latency.observe(&[], 5.0);
        let text = registry.export_prometheus();
        assert!(text.contains("business_latency_seconds_bucket{le=\"0.1\"} 1"));
        assert!(text.contains("business_latency_seconds_bucket{le=\"1\"} 2"));
        assert!(text.contains("business_latency_seconds_bucket{le=\"10\"} 3"));
        assert!(text.contains("business_latency_seconds_bucket{le=\"+Inf\"} 3"));
        // sum 为浮点累加（0.05+0.5+5.0≈5.55），断言可解析性而非精确字符串
        let sum_line = text
            .lines()
            .find(|l| l.starts_with("business_latency_seconds_sum"))
            .unwrap();
        let sum: f64 = sum_line.split_whitespace().last().unwrap().parse().unwrap();
        assert!((sum - 5.55).abs() < 1e-9, "sum 应为 5.55，实际 {sum}");
        assert!(text.contains("business_latency_seconds_count 3"));
    }

    /// 重注册同名且参数一致：幂等；类型冲突 panic。
    #[test]
    fn reregister_idempotent_but_kind_conflict_panics() {
        let registry = MetricsRegistry::new();
        let _ = registry.register_counter_vec("m_total", "help", &["a"]);
        let _ = registry.register_counter_vec("m_total", "help", &["a"]); // 幂等
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = registry.register_gauge_vec("m_total", "help", &["a"]);
        }));
        assert!(result.is_err(), "同名不同类型应 panic");
    }

    /// 无标签指标：导出行不带 `{}`。
    #[test]
    fn unlabeled_counter_export_has_no_braces() {
        let registry = MetricsRegistry::new();
        let total = registry.register_counter_vec("plain_total", "Plain", &[]);
        total.child(&[]).inc_by(9);
        let text = registry.export_prometheus();
        assert!(text.contains("plain_total 9"));
    }

    /// 标签值转义：反斜杠/引号/换行。
    #[test]
    fn label_values_are_escaped() {
        let registry = MetricsRegistry::new();
        let metrics = registry.register_counter_vec("esc_total", "Esc", &["reason"]);
        metrics.child(&["a\"b\\c\nd"]).inc();
        let text = registry.export_prometheus();
        assert!(text.contains("esc_total{reason=\"a\\\"b\\\\c\\nd\"} 1"));
    }
}
