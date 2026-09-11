// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T507：Sink 级日志采样器。
//!
//! [`Sampler`] 决策规则（按顺序短路）：
//! 1. **关键词白名单豁免**：message/target 命中白名单（大小写不敏感）→ 放行；
//! 2. **级别阈值**：级别序 ≥ 阈值（更严重，如阈值 `warn` 时 ERROR/FATAL）→ 放行；
//! 3. **N 取 1**：其余记录按原子计数器每 N 条放行 1 条。
//!
//! [`SamplingSink`] 是 [`LogSink`] 装饰器：采样淘汰的记录不写入内层 sink
//! （计入 `logs_dropped`），其余语义（flush/shutdown/is_healthy）透传。
//! 经 `LoggerBuilder::add_sink` 注册即可给任意第三方 sink 加采样。
//!
//! # Example
//!
//! ```
//! use inklog::support::io::sink::sampling::{SamplingSink, Sampler};
//! use inklog::support::io::{ConsoleSink, LogSink};
//! use inklog::LogTemplate;
//! use std::sync::Arc;
//!
//! let sampler = Sampler::new("info", 10, vec!["critical".to_string()]).unwrap();
//! let inner: Arc<dyn inklog::LogSink> = Arc::new(ConsoleSink::new(
//!     Default::default(),
//!     LogTemplate::new("{timestamp} [{level}] {message}"),
//! ));
//! let sink: Arc<dyn inklog::LogSink> = Arc::new(SamplingSink::new(inner, Arc::new(sampler)));
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::support::query::level_rank;
use crate::{InklogError, LogRecord, Metrics};

/// 采样决策器：级别阈值 + N 取 1 + 关键词白名单豁免。
#[derive(Debug)]
pub struct Sampler {
    /// 级别阈值序数（≥ 该序数的记录直接放行）
    min_level_rank: u8,
    /// N 取 1（低于阈值的记录每 N 条放行 1 条；N=1 表示全放行）
    sample_every_n: u64,
    /// 关键词白名单（大小写不敏感子串匹配 message/target）
    keyword_whitelist: Vec<String>,
    /// 无锁采样计数器
    counter: AtomicU64,
}

impl Sampler {
    /// 创建采样器。
    ///
    /// # Arguments
    ///
    /// * `min_level` - 级别阈值（trace/debug/info/warn/error/fatal，大小写不敏感）
    /// * `sample_every_n` - 低于阈值的记录 N 取 1（≥1）
    /// * `keyword_whitelist` - 豁免关键词（命中即放行）
    ///
    /// # Errors
    ///
    /// 非法级别或 `sample_every_n == 0` 返回 `InklogError::ConfigError`。
    pub fn new(
        min_level: &str,
        sample_every_n: u64,
        keyword_whitelist: Vec<String>,
    ) -> Result<Self, InklogError> {
        if sample_every_n == 0 {
            return Err(InklogError::ConfigError(
                "sample_every_n must be >= 1".to_string(),
            ));
        }
        let rank = level_rank(min_level);
        if rank == u8::MAX {
            return Err(InklogError::ConfigError(format!(
                "Invalid sampling level '{}'. Valid: trace/debug/info/warn/error/fatal",
                min_level
            )));
        }
        Ok(Self {
            min_level_rank: rank,
            sample_every_n,
            keyword_whitelist: keyword_whitelist
                .into_iter()
                .map(|k| k.to_lowercase())
                .collect(),
            counter: AtomicU64::new(0),
        })
    }

    /// 采样决策：true = 写入 sink。
    pub fn should_emit(&self, record: &LogRecord) -> bool {
        // 1. 关键词白名单豁免
        if !self.keyword_whitelist.is_empty() {
            let message = record.message.to_lowercase();
            let target = record.target.to_lowercase();
            if self
                .keyword_whitelist
                .iter()
                .any(|kw| message.contains(kw.as_str()) || target.contains(kw.as_str()))
            {
                return true;
            }
        }
        // 2. 级别阈值
        if level_rank(&record.level) >= self.min_level_rank {
            return true;
        }
        // 3. N 取 1（fetch_add 后取余：第 0 条放行）
        if self.sample_every_n == 1 {
            return true;
        }
        self.counter.fetch_add(1, Ordering::Relaxed) % self.sample_every_n == 0
    }

    /// 采样器生效参数（诊断用）。
    pub fn config(&self) -> (&'static str, u64, &[String]) {
        // 反查级别名仅用于诊断；以秩映射回固定表
        let name = match self.min_level_rank {
            0 => "TRACE",
            1 => "DEBUG",
            2 => "INFO",
            3 => "WARN",
            4 => "ERROR",
            _ => "FATAL",
        };
        (name, self.sample_every_n, &self.keyword_whitelist)
    }
}

/// Sink 级采样装饰器：采样淘汰的记录不写入内层 sink。
pub struct SamplingSink {
    inner: Arc<dyn LogSink>,
    sampler: Arc<Sampler>,
    metrics: Option<Arc<Metrics>>,
}

impl SamplingSink {
    pub fn new(inner: Arc<dyn LogSink>, sampler: Arc<Sampler>) -> Self {
        Self { inner, sampler, metrics: None }
    }

    /// 绑定指标（采样淘汰计入 `logs_dropped`）。
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

#[async_trait]
impl LogSink for SamplingSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        if !self.sampler.should_emit(record) {
            if let Some(ref metrics) = self.metrics {
                metrics.inc_logs_dropped();
            }
            return Ok(());
        }
        self.inner.write(record).await
    }

    async fn flush(&self) -> Result<(), InklogError> {
        self.inner.flush().await
    }

    fn is_healthy(&self) -> bool {
        self.inner.is_healthy()
    }

    async fn shutdown(&self) -> Result<(), InklogError> {
        self.inner.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogRecord;
    use parking_lot::Mutex;
    use tracing::Level;

    fn record(level: Level, target: &str, message: &str) -> LogRecord {
        LogRecord::new(level, target.to_string(), message.to_string())
    }

    #[test]
    fn test_invalid_sampler_config_rejected() {
        assert!(Sampler::new("info", 0, vec![]).is_err(), "N=0 must be rejected");
        assert!(Sampler::new("verbose", 5, vec![]).is_err(), "bad level must be rejected");
    }

    #[test]
    fn test_level_threshold_passes_severe_records() {
        let sampler = Sampler::new("warn", 1000, vec![]).unwrap();
        assert!(sampler.should_emit(&record(Level::ERROR, "t", "x")));
        assert!(sampler.should_emit(&record(Level::WARN, "t", "x")));
        // 低于阈值的记录进入 N 取 1：1000 条 INFO 恰好放行 1 条（1/1000）
        let kept = (0..1000)
            .filter(|_| sampler.should_emit(&record(Level::INFO, "t", "x")))
            .count();
        assert_eq!(kept, 1, "1-in-1000 sampling for below-threshold records");
    }

    #[test]
    fn test_n_of_one_sampling_keeps_every_nth() {
        let sampler = Sampler::new("error", 5, vec![]).unwrap();
        let mut kept = 0;
        for i in 0..100 {
            let mut r = record(Level::INFO, "t", &format!("m{i}"));
            // 固定 message 差异避免白名单语义干扰（白名单为空）
            r.message = format!("m{i}");
            if sampler.should_emit(&r) {
                kept += 1;
            }
        }
        assert_eq!(
            kept,
            20,
            "1-in-5 sampling must keep exactly 20 of 100 below-threshold records"
        );
        // 第 0 条放行：显式验证取余语义
        let s2 = Sampler::new("error", 2, vec![]).unwrap();
        assert!(s2.should_emit(&record(Level::INFO, "t", "first")), "counter 0 passes");
    }

    #[test]
    fn test_keyword_whitelist_exempts_from_sampling() {
        let sampler = Sampler::new("error", 1000, vec!["payment".to_string()]).unwrap();
        // 低于阈值但命中白名单 → 放行
        assert!(sampler.should_emit(&record(
            Level::DEBUG,
            "app::billing",
            "payment processed"
        )));
        // target 命中也放行（大小写不敏感）
        assert!(sampler.should_emit(&record(Level::DEBUG, "PaymentService", "x")));
        // 白名单外的低于阈值记录 → 采样淘汰（首条因计数器 0 放行，次条被淘汰）
        assert!(sampler.should_emit(&record(Level::DEBUG, "app::net", "unrelated first")));
        assert!(!sampler.should_emit(&record(Level::DEBUG, "app::net", "unrelated")));
    }

    /// 捕获型内层 sink。
    struct CollectingSink {
        messages: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl LogSink for CollectingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            self.messages.lock().push(record.message.clone());
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            Ok(())
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sampling_sink_decorator_filters_and_delegates() {
        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
        });
        let sampler = Arc::new(Sampler::new("warn", 1, vec!["critical".to_string()]).unwrap());
        let metrics = Arc::new(Metrics::new());
        let sink = Arc::new(
            SamplingSink::new(inner.clone() as Arc<dyn LogSink>, sampler)
                .with_metrics(metrics.clone()),
        );

        let _ = sink.write(&record(Level::ERROR, "t", "severe")).await;
        let _ = sink.write(&record(Level::INFO, "t", "noise 1")).await;
        let _ = sink.write(&record(Level::INFO, "t", "critical path")).await;
        let _ = sink.write(&record(Level::INFO, "t", "noise 2")).await;

        let messages = inner.messages.lock();
        // ERROR 放行；critical 白名单放行；INFO 噪声（N=1 时全部放行？——
        // N=1 语义为"全放行"，故断言全部写入）
        assert_eq!(messages.len(), 4, "N=1 means pass-through for below-threshold");
        assert!(messages.contains(&"critical path".to_string()));
        drop(messages);

        // 采样淘汰计数：改用 N=1000 的采样器验证淘汰与 metrics
        let inner2 = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
        });
        let sampler2 = Arc::new(Sampler::new("warn", 1000, vec![]).unwrap());
        let sink2 = Arc::new(
            SamplingSink::new(inner2.clone() as Arc<dyn LogSink>, sampler2)
                .with_metrics(metrics.clone()),
        );
        // 首条放行（计数器 0），第二条被采样淘汰
        let _ = sink2.write(&record(Level::INFO, "t", "first passes")).await;
        let _ = sink2.write(&record(Level::INFO, "t", "dropped")).await;
        assert_eq!(
            inner2.messages.lock().len(),
            1,
            "only the first below-threshold record may reach the inner sink"
        );
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "sampled-out record must count as dropped"
        );
    }

    #[tokio::test]
    async fn test_sampling_sink_forwards_flush_shutdown_and_health() {
        struct HealthSink {
            healthy: std::sync::atomic::AtomicBool,
            shutdown_called: std::sync::atomic::AtomicBool,
        }
        #[async_trait]
        impl LogSink for HealthSink {
            async fn write(&self, _r: &LogRecord) -> Result<(), InklogError> {
                Ok(())
            }
            async fn flush(&self) -> Result<(), InklogError> {
                Ok(())
            }
            fn is_healthy(&self) -> bool {
                self.healthy.load(Ordering::Relaxed)
            }
            async fn shutdown(&self) -> Result<(), InklogError> {
                self.shutdown_called.store(true, Ordering::Relaxed);
                Ok(())
            }
        }

        let inner = Arc::new(HealthSink {
            healthy: std::sync::atomic::AtomicBool::new(true),
            shutdown_called: std::sync::atomic::AtomicBool::new(false),
        });
        let sampler = Arc::new(Sampler::new("info", 1, vec![]).unwrap());
        let sink = SamplingSink::new(inner.clone() as Arc<dyn LogSink>, sampler);
        assert!(sink.is_healthy(), "health must be forwarded");
        inner
            .healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);
        assert!(!sink.is_healthy());
        sink.shutdown().await.unwrap();
        assert!(
            inner
                .shutdown_called
                .load(std::sync::atomic::Ordering::Relaxed),
            "shutdown must be delegated"
        );
    }
}
