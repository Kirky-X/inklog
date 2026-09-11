// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T511：按目标的 sink 写入限流端口（防日志洪水）。
//!
//! 分层铁律：inklog（下层）只定义端口 + NoOp 默认实现；token 预算/滑动窗口
//! 等真实算法由上层项目（limiteron，见其 T617）实现 [`SinkRateLimit`] 后经
//! [`LoggerBuilder::add_sink`](crate::LoggerBuilder::add_sink) 注入
//! [`RateLimitedSink`] 装饰器。
//!
//! 端口为对象安全 trait（`dyn` 可用）：决策点在 sink 写入前同步执行——
//! 返回 `true` 表示预算允许本次写入；写入结果经 [`SinkRateLimit::report`]
//! 回报，供预算回收或上层熔断。
//!
//! # Example
//!
//! ```ignore
//! // limiteron 侧：
//! struct LimiteronRateLimit { /* token bucket per target */ }
//! impl SinkRateLimit for LimiteronRateLimit { /* ... */ }
//!
//! // inklog 侧注入：
//! let sink = RateLimitedSink::new(inner_sink, Arc::new(LimiteronRateLimit::new(config)));
//! LoggerManager::builder().add_sink(Arc::new(sink));
//! ```

use std::sync::Arc;
use std::sync::atomic::Ordering;

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::{InklogError, LogRecord, Metrics};

/// sink 写入结果（供限流端口回报预算/熔断状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkWriteOutcome {
    /// 记录已写入
    Written,
    /// 被限流端口拒绝（未写入）
    Rejected,
    /// 写入失败
    Failed,
}

/// 按目标的 sink 写入限流端口（对象安全，NoOp 默认放行）。
///
/// 下层默认实现 [`NoOpRateLimit`]；上层 limiteron 提供真实 token 预算实现。
pub trait SinkRateLimit: Send + Sync {
    /// 记录写入前判定；`true` = 允许本次写入（消费预算）。
    ///
    /// `record` 携带 target/level/fields，供按目标（target 前缀/租户键）
    /// 分桶决策。
    fn try_acquire(&self, record: &LogRecord) -> bool;

    /// 写入结果回报（默认 no-op）。
    fn report(&self, _record: &LogRecord, _outcome: SinkWriteOutcome) {}

    /// 端口实现名（诊断/指标标签）。
    fn name(&self) -> &str {
        "sink-rate-limit"
    }
}

/// 默认实现：恒放行、零开销。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOpRateLimit;

impl SinkRateLimit for NoOpRateLimit {
    fn try_acquire(&self, _record: &LogRecord) -> bool {
        true
    }
}

/// 简单的全局令牌桶实现（端口语义验证与测试基线；生产推荐 limiteron）。
pub struct TokenBucketRateLimit {
    tokens: std::sync::atomic::AtomicU64,
    capacity: u64,
    name: String,
}

impl TokenBucketRateLimit {
    /// 以固定容量创建（无后台补充线程；`refill` 可在 report 中按需扩展）。
    pub fn new(capacity: u64) -> Self {
        Self {
            tokens: std::sync::atomic::AtomicU64::new(capacity),
            capacity,
            name: format!("token-bucket({capacity})"),
        }
    }

    /// 当前剩余预算。
    pub fn available(&self) -> u64 {
        self.tokens.load(Ordering::Relaxed)
    }

    /// 归还预算上限。
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

impl SinkRateLimit for TokenBucketRateLimit {
    fn try_acquire(&self, _record: &LogRecord) -> bool {
        // CAS 扣减，避免超卖
        let mut current = self.tokens.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return false;
            }
            match self.tokens.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    fn report(&self, _record: &LogRecord, outcome: SinkWriteOutcome) {
        // 被拒绝的记录不消费预算；写失败归还预算（下次重试有额度）
        if outcome == SinkWriteOutcome::Failed {
            let mut current = self.tokens.load(Ordering::Relaxed);
            while current < self.capacity {
                match self.tokens.compare_exchange_weak(
                    current,
                    current + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(actual) => current = actual,
                }
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Sink 级限流装饰器：写入前经 [`SinkRateLimit`] 判定，拒绝计 `logs_dropped`。
pub struct RateLimitedSink {
    inner: Arc<dyn LogSink>,
    limiter: Arc<dyn SinkRateLimit>,
    metrics: Option<Arc<Metrics>>,
}

impl RateLimitedSink {
    pub fn new(inner: Arc<dyn LogSink>, limiter: Arc<dyn SinkRateLimit>) -> Self {
        Self { inner, limiter, metrics: None }
    }

    /// 绑定指标（拒绝计入 `logs_dropped`）。
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

#[async_trait]
impl LogSink for RateLimitedSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        if !self.limiter.try_acquire(record) {
            if let Some(ref metrics) = self.metrics {
                metrics.inc_logs_dropped();
            }
            self.limiter.report(record, SinkWriteOutcome::Rejected);
            return Ok(());
        }
        match self.inner.write(record).await {
            Ok(()) => {
                self.limiter.report(record, SinkWriteOutcome::Written);
                Ok(())
            }
            Err(e) => {
                self.limiter.report(record, SinkWriteOutcome::Failed);
                Err(e)
            }
        }
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
    use parking_lot::Mutex;

    fn record(level: tracing::Level, target: &str) -> LogRecord {
        LogRecord::new(level, target.to_string(), "msg".to_string())
    }

    #[test]
    fn test_noop_rate_limit_always_allows() {
        let limiter = NoOpRateLimit;
        for i in 0..1000 {
            assert!(
                limiter.try_acquire(&record(tracing::Level::INFO, &format!("t{i}"))),
                "NoOp must always allow"
            );
        }
        assert_eq!(limiter.name(), "sink-rate-limit");
    }

    #[test]
    fn test_token_bucket_budget_and_refund() {
        let limiter = TokenBucketRateLimit::new(3);
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert_eq!(limiter.available(), 0);
        assert!(
            !limiter.try_acquire(&record(tracing::Level::ERROR, "t")),
            "exhausted budget must reject (even severe records; per-target policy is upper-layer)"
        );
        // 写失败归还预算
        limiter.report(&record(tracing::Level::INFO, "t"), SinkWriteOutcome::Failed);
        assert_eq!(limiter.available(), 1);
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
    }

    /// 捕获型内层 sink。
    struct CollectingSink {
        messages: Mutex<Vec<String>>,
        fail_once: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl LogSink for CollectingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            if self
                .fail_once
                .swap(false, Ordering::Relaxed)
            {
                return Err(InklogError::ConfigError("simulated failure".to_string()));
            }
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
    async fn test_rate_limited_sink_blocks_writes_and_counts_dropped() {
        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
            fail_once: std::sync::atomic::AtomicBool::new(false),
        });
        let metrics = Arc::new(Metrics::new());
        let sink = Arc::new(
            RateLimitedSink::new(inner.clone() as Arc<dyn LogSink>, Arc::new(TokenBucketRateLimit::new(2)))
                .with_metrics(metrics.clone()),
        );

        let _ = sink.write(&record(tracing::Level::INFO, "t")).await.unwrap();
        let _ = sink.write(&record(tracing::Level::INFO, "t")).await.unwrap();
        let _ = sink.write(&record(tracing::Level::INFO, "t")).await.unwrap(); // 被拒

        assert_eq!(inner.messages.lock().len(), 2, "budget=2 → exactly 2 writes");
        assert_eq!(metrics.logs_dropped(), 1, "rejected record must count as dropped");
    }

    #[tokio::test]
    async fn test_rate_limited_sink_reports_failure_for_budget_refund() {
        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
            fail_once: std::sync::atomic::AtomicBool::new(true),
        });
        let limiter = Arc::new(TokenBucketRateLimit::new(2));
        let sink = RateLimitedSink::new(inner.clone() as Arc<dyn LogSink>, limiter.clone());

        // 首条：预算扣 1，写失败 → 预算归还
        let r = sink.write(&record(tracing::Level::INFO, "t")).await;
        assert!(r.is_err(), "inner failure must propagate");
        assert_eq!(limiter.available(), 2, "failed write must refund the budget");
        // 次条：写成功 → 消耗 1
        let _ = sink.write(&record(tracing::Level::INFO, "t")).await.unwrap();
        assert_eq!(limiter.available(), 1);
        assert_eq!(inner.messages.lock().len(), 1);
    }

    /// 端口对象安全性验证：dyn SinkRateLimit 可经 Arc 分发（limiteron 注入形态）。
    #[test]
    fn test_port_is_object_safe() {
        fn accepts(_limiter: Arc<dyn SinkRateLimit>) {}
        accepts(Arc::new(NoOpRateLimit));
        accepts(Arc::new(TokenBucketRateLimit::new(1)));
    }
}
