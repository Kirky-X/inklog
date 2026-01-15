use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

#[derive(Debug, Serialize, Clone)]
pub struct SinkHealth {
    pub healthy: bool,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
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
    pub overall: bool,
    pub sinks: HashMap<String, SinkHealth>,
    pub channel_usage: f64,
    pub uptime_seconds: u64,
    pub metrics: MetricsSnapshot,
}

#[derive(Debug)]
pub struct Metrics {
    pub logs_written_total: AtomicU64,
    pub logs_dropped_total: AtomicU64,
    pub channel_send_blocked_total: AtomicU64,
    pub sink_errors_total: AtomicU64,
    pub start_time: Instant,

    // Latency tracking
    pub total_latency_us: AtomicU64,
    pub latency_count: AtomicU64,
    pub latency_histogram: Histogram,

    // Gauges
    pub active_workers: Gauge,

    // Sink Health
    pub sink_health: Mutex<HashMap<String, SinkHealth>>,
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

    pub fn update_sink_health(&self, name: &str, healthy: bool, error: Option<String>) {
        if let Ok(mut map) = self.sink_health.lock() {
            let entry = map.entry(name.to_string()).or_insert(SinkHealth {
                healthy: true,
                last_error: None,
                consecutive_failures: 0,
            });

            entry.healthy = healthy;
            if !healthy {
                entry.consecutive_failures += 1;
                entry.last_error = error;
            } else {
                entry.consecutive_failures = 0;
                entry.last_error = None;
            }
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_status(&self, channel_len: usize, channel_cap: usize) -> HealthStatus {
        let sinks: std::collections::HashMap<String, SinkHealth> = match self.sink_health.lock() {
            Ok(guard) => guard.clone(),
            Err(_e) => {
                eprintln!("Metrics mutex poisoned, using empty data");
                std::collections::HashMap::new()
            }
        };
        let overall = sinks.values().all(|s| s.healthy);

        let count = self.latency_count.load(Ordering::Relaxed);
        let total = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency = if count > 0 { total / count } else { 0 };

        HealthStatus {
            overall,
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
                let value = if health.healthy { 1 } else { 0 };
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
