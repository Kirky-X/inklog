// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Sink 级日志采样器。
//!
//! [`Sampler`] 决策规则（按顺序短路）：
//! 1. **关键词白名单豁免**：message/target 命中白名单（大小写不敏感）→ 放行；
//! 2. **级别阈值**：级别序 ≥ 阈值（更严重，如阈值 `warn` 时 ERROR/FATAL）→ 放行；
//! 3. **N 取 1**：其余记录按原子计数器每 N 条放行 1 条。
//!
//! [`SamplingSink`] 是 [`LogSink`] 装饰器：采样淘汰的记录不写入内层 sink
//! （计入 `logs_dropped` 与 `sampled_out_total`），其余语义
//! （flush/shutdown/is_healthy）透传。
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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;

use crate::domain::config::sampling::SamplingConfig;
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

    /// 纯 N 取 1 采样器：无级别豁免阈值，所有记录进入计数器。
    ///
    /// [`SamplingPolicy`] 的 per_level 采样率与无 `keep_level` 的前缀规则
    /// 以此为基座（级别路由由策略层完成，采样器只承担计数）。
    ///
    /// # Errors
    ///
    /// `sample_every_n == 0` 返回 [`InklogError::ConfigError`]。
    pub(crate) fn n_of_one(sample_every_n: u64) -> Result<Self, InklogError> {
        if sample_every_n == 0 {
            return Err(InklogError::ConfigError(
                "sample_every_n must be >= 1".to_string(),
            ));
        }
        // 阈值取不可能达到的秩（u8::MAX），使任何合法记录都落入计数器路径
        Ok(Self {
            min_level_rank: u8::MAX,
            sample_every_n,
            keyword_whitelist: Vec::new(),
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
        self.counter
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(self.sample_every_n)
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

/// 限流压力下的采样策略（Subscriber 级）：per_level 采样率 + per_target_prefix 规则。
///
/// 每条规则编译为一个 [`Sampler`]（决策基座复用：keep_level 阈值豁免 +
/// N 取 1，计数器 0 放行，N=1 全放行）。[`SamplingPolicy::should_emit`]
/// 返回 `None` 表示无规则命中（含空策略），调用方回退内置兜底采样——
/// 只配置部分规则时，未提及的记录仍按既有压力语义（非关键丢弃、
/// ERROR/FATAL 1-in-N）处理，不因引入策略而意外丢失。
pub struct SamplingPolicy {
    /// 级别秩 → 纯 N 取 1 采样器（级别路由由本表完成，无豁免阈值）
    per_level: HashMap<u8, Sampler>,
    /// (小写 target 前缀, 规则采样器)；按前缀长度降序排列，首个命中生效
    per_target_prefix: Vec<(String, Sampler)>,
}

impl SamplingPolicy {
    /// 从采样配置构建策略。
    ///
    /// # Errors
    ///
    /// 级别名非法、采样率为 0、前缀为空或别名键（`warn`/`warning` 等）
    /// 映射到同一级别时返回 [`InklogError::ConfigError`]。
    pub fn from_config(config: &SamplingConfig) -> Result<Self, InklogError> {
        let mut per_level = HashMap::with_capacity(config.per_level.len());
        for (level, rate) in &config.per_level {
            let rank = level_rank(level);
            if rank == u8::MAX {
                return Err(InklogError::ConfigError(format!(
                    "Invalid sampling level '{level}'. Valid: trace/debug/info/warn/error/fatal"
                )));
            }
            if *rate == 0 {
                return Err(InklogError::ConfigError(format!(
                    "sampling rate for level '{level}' must be >= 1"
                )));
            }
            if per_level.keys().any(|r| *r == rank) {
                return Err(InklogError::ConfigError(format!(
                    "sampling levels '{level}' and a rank-equivalent alias both configured; \
                     keep one per level rank"
                )));
            }
            // 级别路由由 per_level 键完成，采样器只承担 N 取 1
            per_level.insert(rank, Sampler::n_of_one(*rate)?);
        }
        let mut per_target_prefix: Vec<(String, Sampler)> =
            Vec::with_capacity(config.per_target_prefix.len());
        for (prefix, rule) in &config.per_target_prefix {
            if prefix.is_empty() {
                return Err(InklogError::ConfigError(
                    "sampling target prefix must not be empty".to_string(),
                ));
            }
            if rule.sample_every_n == 0 {
                return Err(InklogError::ConfigError(format!(
                    "sample_every_n for prefix '{prefix}' must be >= 1"
                )));
            }
            let sampler = match rule.keep_level.as_deref() {
                Some(keep_level) => {
                    if level_rank(keep_level) == u8::MAX {
                        return Err(InklogError::ConfigError(format!(
                            "Invalid keep_level '{keep_level}'. \
                             Valid: trace/debug/info/warn/error/fatal"
                        )));
                    }
                    Sampler::new(keep_level, rule.sample_every_n, Vec::new())?
                }
                // 未配置豁免阈值：纯 N 取 1
                None => Sampler::n_of_one(rule.sample_every_n)?,
            };
            per_target_prefix.push((prefix.to_lowercase(), sampler));
        }
        // 最长前缀优先：更具体的规则胜出
        per_target_prefix.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
        Ok(Self {
            per_level,
            per_target_prefix,
        })
    }

    /// 是否未配置任何规则（未配置 = 调用方走内置兜底采样）。
    pub fn is_empty(&self) -> bool {
        self.per_level.is_empty() && self.per_target_prefix.is_empty()
    }

    /// 采样决策：`Some(true)` 放行、`Some(false)` 采样淘汰、`None` 无规则
    /// 命中（调用方回退内置兜底采样）。
    ///
    /// 命中顺序：target 前缀规则（最长前缀优先，大小写不敏感）→
    /// 每级别采样率。级别别名（warning/critical）与主名等价。
    pub fn should_emit(&self, record: &LogRecord) -> Option<bool> {
        let target = record.target.to_lowercase();
        for (prefix, sampler) in &self.per_target_prefix {
            if target.starts_with(prefix.as_str()) {
                return Some(sampler.should_emit(record));
            }
        }
        let rank = level_rank(&record.level);
        self.per_level.get(&rank).map(|s| s.should_emit(record))
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
        Self {
            inner,
            sampler,
            metrics: None,
        }
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
                metrics.inc_sampled_out();
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
        assert!(
            Sampler::new("info", 0, vec![]).is_err(),
            "N=0 must be rejected"
        );
        assert!(
            Sampler::new("verbose", 5, vec![]).is_err(),
            "bad level must be rejected"
        );
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
            kept, 20,
            "1-in-5 sampling must keep exactly 20 of 100 below-threshold records"
        );
        // 第 0 条放行：显式验证取余语义
        let s2 = Sampler::new("error", 2, vec![]).unwrap();
        assert!(
            s2.should_emit(&record(Level::INFO, "t", "first")),
            "counter 0 passes"
        );
    }

    #[test]
    fn test_keyword_whitelist_exempts_from_sampling() {
        let sampler = Sampler::new("error", 1000, vec!["payment".to_string()]).unwrap();
        // 低于阈值但命中白名单 → 放行
        assert!(sampler.should_emit(&record(Level::DEBUG, "app::billing", "payment processed")));
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

    /// 计数 flush/shutdown 调用的探针：验证 SamplingSink 把生命周期调用
    /// 真实委托给内层 sink（CollectingSink 的空实现观测不到转发）。
    struct LifecycleProbeSink {
        flushes: AtomicU64,
        shutdowns: AtomicU64,
    }

    #[async_trait]
    impl LogSink for LifecycleProbeSink {
        async fn write(&self, _record: &LogRecord) -> Result<(), InklogError> {
            Ok(())
        }
        async fn flush(&self) -> Result<(), InklogError> {
            self.flushes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            self.shutdowns.fetch_add(1, Ordering::Relaxed);
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

        // ERROR 放行；critical 白名单放行；INFO 噪声（N=1 时全部放行？——
        // N=1 语义为"全放行"，故断言全部写入）
        {
            let messages = inner.messages.lock();
            assert_eq!(
                messages.len(),
                4,
                "N=1 means pass-through for below-threshold"
            );
            assert!(messages.contains(&"critical path".to_string()));
        }

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
        assert_eq!(
            metrics.sampled_out(),
            1,
            "sampled-out record must count in the dedicated sampling metric"
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

    fn policy_record(level: Level, target: &str, message: &str) -> LogRecord {
        LogRecord::new(level, target.to_string(), message.to_string())
    }

    fn policy_from(
        per_level: &[(&str, u64)],
        per_target_prefix: &[(&str, Option<&str>, u64)],
    ) -> SamplingPolicy {
        let mut config = SamplingConfig::default();
        for (level, rate) in per_level {
            config.per_level.insert(level.to_string(), *rate);
        }
        for (prefix, keep_level, rate) in per_target_prefix {
            config.per_target_prefix.insert(
                prefix.to_string(),
                crate::domain::config::sampling::TargetSamplingRule {
                    keep_level: keep_level.map(str::to_string),
                    sample_every_n: *rate,
                },
            );
        }
        SamplingPolicy::from_config(&config).unwrap()
    }

    #[test]
    fn test_policy_from_config_rejects_invalid_rules() {
        let mut config = SamplingConfig::default();
        config.per_level.insert("info".to_string(), 0);
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "zero rate must be rejected"
        );

        let mut config = SamplingConfig::default();
        config.per_level.insert("verbose".to_string(), 5);
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "invalid level key must be rejected"
        );

        let mut config = SamplingConfig::default();
        config.per_level.insert("warn".to_string(), 5);
        config.per_level.insert("warning".to_string(), 10);
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "alias keys mapping to the same rank must be rejected"
        );

        let mut config = SamplingConfig::default();
        config.per_target_prefix.insert(
            String::new(),
            crate::domain::config::sampling::TargetSamplingRule::default(),
        );
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "empty prefix must be rejected"
        );

        let mut config = SamplingConfig::default();
        config.per_target_prefix.insert(
            "app::x".to_string(),
            crate::domain::config::sampling::TargetSamplingRule {
                keep_level: Some("verbose".to_string()),
                sample_every_n: 1,
            },
        );
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "invalid keep_level must be rejected"
        );

        let mut config = SamplingConfig::default();
        config.per_target_prefix.insert(
            "app::x".to_string(),
            crate::domain::config::sampling::TargetSamplingRule {
                keep_level: None,
                sample_every_n: 0,
            },
        );
        assert!(
            SamplingPolicy::from_config(&config).is_err(),
            "zero rule rate must be rejected"
        );
    }

    #[test]
    fn test_policy_default_config_is_empty_and_opinionless() {
        let policy = SamplingPolicy::from_config(&SamplingConfig::default()).unwrap();
        assert!(policy.is_empty());
        assert_eq!(
            policy.should_emit(&policy_record(Level::ERROR, "any", "x")),
            None,
            "empty policy must express no opinion so callers fall back to baseline"
        );
    }

    #[test]
    fn test_policy_per_level_n_of_one_sampling() {
        let policy = policy_from(&[("info", 5)], &[]);
        let kept = (0..100)
            .filter(|i| {
                policy.should_emit(&policy_record(Level::INFO, "t", &format!("m{i}"))) == Some(true)
            })
            .count();
        assert_eq!(
            kept, 20,
            "1-in-5 per-level rate must keep exactly 20 of 100 records"
        );
        // 未配置级别的记录无规则命中 → 无意见
        assert_eq!(
            policy.should_emit(&policy_record(Level::DEBUG, "t", "x")),
            None,
            "levels without a rule must yield None"
        );
    }

    #[test]
    fn test_policy_prefix_rules_longest_prefix_wins() {
        let policy = policy_from(
            &[],
            &[
                ("app", Some("error"), 10),
                ("app::audit", Some("debug"), 10),
            ],
        );
        // 长前缀命中：keep_level=debug 豁免 INFO
        assert!(
            policy.should_emit(&policy_record(Level::INFO, "app::audit::core", "x")) == Some(true),
            "longest matching prefix rule must win"
        );
        // 短前缀命中：keep_level=error，INFO 未豁免且 n=10 → 计数器 0 放行
        assert!(
            policy.should_emit(&policy_record(Level::INFO, "app::net", "first")) == Some(true),
            "counter 0 must pass through the rule sampler"
        );
    }

    #[test]
    fn test_policy_prefix_match_is_case_insensitive() {
        let policy = policy_from(&[], &[("app::audit", Some("debug"), 10)]);
        assert!(
            policy.should_emit(&policy_record(Level::INFO, "App::Audit::Core", "x")) == Some(true),
            "target prefix matching must be case-insensitive"
        );
    }

    #[test]
    fn test_policy_prefix_keep_level_exempts_severe_records_only() {
        // keep_level=warn：WARN 及以上豁免全放行；TRACE 未豁免 → n=1 全放行；
        // 换 n=2 时 TRACE 走 N 取 1
        let policy = policy_from(&[], &[("app::audit", Some("warn"), 1)]);
        assert!(policy.should_emit(&policy_record(Level::ERROR, "app::audit", "x")) == Some(true));
        assert!(policy.should_emit(&policy_record(Level::WARN, "app::audit", "x")) == Some(true));

        let policy = policy_from(&[], &[("app::audit", Some("warn"), 2)]);
        let kept = (0..10)
            .filter(|i| {
                policy.should_emit(&policy_record(Level::TRACE, "app::audit", &format!("t{i}")))
                    == Some(true)
            })
            .count();
        assert_eq!(
            kept, 5,
            "records below keep_level must follow the N-of-1 rate"
        );
    }

    #[test]
    fn test_policy_prefix_rule_takes_precedence_over_per_level() {
        let policy = policy_from(&[("info", 100)], &[("app::audit", Some("trace"), 1)]);
        // 前缀规则命中 → 规则采样器（n=1 全放行），而非 info 的 1-in-100
        for i in 0..10 {
            assert!(
                policy.should_emit(&policy_record(Level::INFO, "app::audit", &format!("m{i}")))
                    == Some(true),
                "prefix rule must take precedence over per-level rate"
            );
        }
    }

    #[test]
    fn test_n_of_one_rejects_zero() {
        let err = Sampler::n_of_one(0).unwrap_err();
        assert!(
            err.to_string().contains("sample_every_n must be >= 1"),
            "got: {err}"
        );
    }

    #[test]
    fn test_sampler_config_accessor_roundtrip() {
        let sampler = Sampler::new("warn", 8, vec!["urgent".to_string()]).expect("valid sampler");
        let (name, every_n, whitelist) = sampler.config();
        assert_eq!(name, "WARN");
        assert_eq!(every_n, 8);
        assert_eq!(whitelist, &["urgent".to_string()]);
    }

    #[tokio::test]
    async fn test_sampling_sink_flush_and_shutdown_forwarded() {
        let inner = Arc::new(LifecycleProbeSink {
            flushes: AtomicU64::new(0),
            shutdowns: AtomicU64::new(0),
        });
        let sampler = Arc::new(Sampler::n_of_one(1).expect("valid sampler"));
        let sink = SamplingSink::new(inner.clone() as Arc<dyn LogSink>, sampler);
        sink.flush().await.unwrap();
        sink.shutdown().await.unwrap();
        assert_eq!(
            inner.flushes.load(Ordering::Relaxed),
            1,
            "flush must be delegated to the inner sink exactly once"
        );
        assert_eq!(
            inner.shutdowns.load(Ordering::Relaxed),
            1,
            "shutdown must be delegated to the inner sink exactly once"
        );
    }
}
