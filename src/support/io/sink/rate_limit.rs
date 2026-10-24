// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 按目标的 sink 写入限流端口（防日志洪水）。
//!
//! 分层铁律：inklog（下层）只定义端口 + NoOp 默认实现；token 预算/滑动窗口
//! 等真实算法由上层项目（limiteron）实现 [`SinkRateLimit`] 后经
//! [`LoggerBuilder::add_sink`](crate::LoggerBuilder::add_sink) 注入
//! [`RateLimitedSink`] 装饰器。
//!
//! 端口为对象安全 trait（`dyn` 可用）：决策点在 sink 写入前同步执行——
//! 返回 `true` 表示预算允许本次写入；写入结果经 [`SinkRateLimit::report`]
//! 回报，供预算回收或上层熔断。
//!
//! 默认路径不依赖 limiteron：突发配额内核由仓内 [`BurstLimiter`] 承担
//! （无时钟、无补充），与 limiteron `TokenBucketLimiter::new(capacity, 0)`
//! 的纯预算语义等价。需要 limiteron 原生限流器（速率补充/按目标分桶）时
//! 启用 `limiteron-rate-limit` feature，经 [`RateLimitedSink::with_limiter`]
//! 注入。
//!
//! # Example
//!
//! ```ignore
//! // 默认路径（无 limiteron 依赖）：
//! let sink = RateLimitedSink::new(inner_sink, Arc::new(TokenBucketRateLimit::new(100)));
//!
//! // limiteron-rate-limit feature：注入 limiteron 限流器
//! let sink = RateLimitedSink::with_limiter(inner_sink, Arc::new(limiter));
//! LoggerManager::builder().add_sink(Arc::new(sink));
//! ```

use std::sync::Arc;

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::support::io::sink::BurstLimiter;
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
/// 下层默认实现 [`NoOpRateLimit`]；纯预算基线实现 [`TokenBucketRateLimit`]
/// （内核为仓内 [`BurstLimiter`]）；上层限流器（按 target 分桶等增强语义）
/// 由消费方实现本端口注入，inklog 不依赖任何第三方限流库。
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

/// 全局突发配额限流（[`SinkRateLimit`] 端口的内置基线实现）。
///
/// 算法内核为仓内 [`BurstLimiter`]（原子 CAS 递减，无时钟、无补充）：
/// `new(capacity)` 与 limiteron `TokenBucketLimiter::new(capacity, 0)`
/// （`refill_rate = 0` 的纯预算语义）行为等价——初始放行 `capacity` 次，
/// 其后拒绝，无时间补充。
///
/// 语义：`report(Failed)` 不归还预算——写失败的预算由 worker 层重试兜底；
/// 归还 API 的库侧需求见 upstream-requests。
pub struct TokenBucketRateLimit {
    limiter: BurstLimiter,
    name: String,
}

impl TokenBucketRateLimit {
    /// 以固定容量创建（纯预算：无后台补充线程，无时间补充）。
    pub fn new(capacity: u64) -> Self {
        Self {
            limiter: BurstLimiter::new(capacity),
            name: format!("token-bucket({capacity})"),
        }
    }

    /// 当前剩余预算。
    pub fn available(&self) -> u64 {
        self.limiter.available()
    }

    /// 归还预算上限。
    pub fn capacity(&self) -> u64 {
        self.limiter.capacity()
    }
}

impl SinkRateLimit for TokenBucketRateLimit {
    fn try_acquire(&self, _record: &LogRecord) -> bool {
        self.limiter.try_acquire()
    }

    fn report(&self, _record: &LogRecord, _outcome: SinkWriteOutcome) {
        // 纯预算内核无归还路径：写失败的预算不归还（语义钉死于测试）
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
        Self {
            inner,
            limiter,
            metrics: None,
        }
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
    use std::sync::atomic::Ordering;

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
    fn test_token_bucket_budget_without_refund() {
        // 纯预算语义（limiteron refill_rate=0）：耗尽后不放行；写失败不再归还
        let limiter = TokenBucketRateLimit::new(3);
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert_eq!(limiter.available(), 0);
        assert!(
            !limiter.try_acquire(&record(tracing::Level::ERROR, "t")),
            "exhausted budget must reject (even severe records; per-target policy is upper-layer)"
        );
        // 写失败不归还预算（limiteron 无 refund API，语义变更钉死）
        limiter.report(&record(tracing::Level::INFO, "t"), SinkWriteOutcome::Failed);
        assert_eq!(
            limiter.available(),
            0,
            "failed write must NOT refund the budget"
        );
        assert!(
            !limiter.try_acquire(&record(tracing::Level::INFO, "t")),
            "budget stays exhausted after a failed write"
        );
    }

    #[test]
    fn test_token_bucket_pure_budget_no_time_refill() {
        // refill_rate=0：经过时间也不补充（纯预算语义由 new(capacity) 的
        // limiteron new(capacity, 0) 映射保证）
        let limiter = TokenBucketRateLimit::new(2);
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert!(limiter.try_acquire(&record(tracing::Level::INFO, "t")));
        assert_eq!(limiter.available(), 0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(limiter.available(), 0, "no time-based refill must occur");
        assert!(!limiter.try_acquire(&record(tracing::Level::INFO, "t")));
    }

    /// 捕获型内层 sink。
    struct CollectingSink {
        messages: Mutex<Vec<String>>,
        fail_once: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl LogSink for CollectingSink {
        async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
            if self.fail_once.swap(false, Ordering::Relaxed) {
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

    /// 计数 flush/shutdown 并携带受控健康态的探针：验证 RateLimitedSink 把
    /// 生命周期调用与健康检查真实委托给内层 sink（CollectingSink 观测不到）。
    struct LifecycleProbeSink {
        healthy: std::sync::atomic::AtomicBool,
        flushes: std::sync::atomic::AtomicUsize,
        shutdowns: std::sync::atomic::AtomicUsize,
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
        fn is_healthy(&self) -> bool {
            self.healthy.load(Ordering::Relaxed)
        }
        async fn shutdown(&self) -> Result<(), InklogError> {
            self.shutdowns.fetch_add(1, Ordering::Relaxed);
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
            RateLimitedSink::new(
                inner.clone() as Arc<dyn LogSink>,
                Arc::new(TokenBucketRateLimit::new(2)),
            )
            .with_metrics(metrics.clone()),
        );

        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap();
        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap();
        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap(); // 被拒

        assert_eq!(
            inner.messages.lock().len(),
            2,
            "budget=2 → exactly 2 writes"
        );
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "rejected record must count as dropped"
        );
    }

    #[tokio::test]
    async fn test_rate_limited_sink_reports_failure_without_budget_refund() {
        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
            fail_once: std::sync::atomic::AtomicBool::new(true),
        });
        let limiter = Arc::new(TokenBucketRateLimit::new(2));
        let sink = RateLimitedSink::new(inner.clone() as Arc<dyn LogSink>, limiter.clone());

        // 首条：预算扣 1，写失败 → 预算不归还（limiteron 无 refund API）
        let r = sink.write(&record(tracing::Level::INFO, "t")).await;
        assert!(r.is_err(), "inner failure must propagate");
        assert_eq!(
            limiter.available(),
            1,
            "failed write must not refund the budget"
        );
        // 次条：写成功 → 再消耗 1
        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap();
        assert_eq!(limiter.available(), 0);
        assert_eq!(inner.messages.lock().len(), 1);
    }

    /// 端口对象安全性验证：dyn SinkRateLimit 可经 Arc 分发（消费方注入形态）。
    #[test]
    fn test_port_is_object_safe() {
        fn accepts(_limiter: Arc<dyn SinkRateLimit>) {}
        accepts(Arc::new(NoOpRateLimit));
        accepts(Arc::new(TokenBucketRateLimit::new(1)));
    }

    /// 自有端口注入路径：sink 级装饰器经消费方实现的 `SinkRateLimit` 裁决——
    /// 预算耗尽后拒绝写入。外部扩展的正式入口（不再暴露第三方限流 trait）。
    #[tokio::test]
    async fn test_port_injection_path_limits_writes() {
        /// 容量 1 的纯预算限流器（无补充）
        struct OneShotLimiter {
            left: std::sync::atomic::AtomicUsize,
        }

        impl SinkRateLimit for OneShotLimiter {
            fn try_acquire(&self, _record: &LogRecord) -> bool {
                self.left
                    .fetch_update(
                        std::sync::atomic::Ordering::AcqRel,
                        std::sync::atomic::Ordering::Acquire,
                        |v| if v > 0 { Some(v - 1) } else { None },
                    )
                    .is_ok()
            }
        }

        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
            fail_once: std::sync::atomic::AtomicBool::new(false),
        });
        let limiter = Arc::new(OneShotLimiter {
            left: std::sync::atomic::AtomicUsize::new(1),
        });
        let sink = RateLimitedSink::new(inner.clone() as Arc<dyn LogSink>, limiter);

        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap();
        sink.write(&record(tracing::Level::INFO, "t"))
            .await
            .unwrap();

        assert_eq!(
            inner.messages.lock().len(),
            1,
            "the injected port limiter must gate the write"
        );
    }

    #[test]
    fn test_noop_report_and_token_bucket_metadata() {
        NoOpRateLimit.report(
            &record(tracing::Level::INFO, "t"),
            SinkWriteOutcome::Written,
        );

        let limiter = TokenBucketRateLimit::new(7);
        assert_eq!(limiter.capacity(), 7);
        assert_eq!(limiter.name(), "token-bucket(7)");
        // 满员时 report(Failed) 不超出容量（不归还语义下 available 恒不变）
        limiter.report(&record(tracing::Level::INFO, "t"), SinkWriteOutcome::Failed);
        assert_eq!(limiter.available(), 7);
        assert_eq!(
            NoOpRateLimit.name(),
            "sink-rate-limit",
            "default name must survive custom override on TokenBucket"
        );
    }

    #[test]
    fn test_token_bucket_cas_survives_contention() {
        // 并发争抢同一桶：CAS 冲突重试后总成功数必须恰等于容量（无超卖）
        let limiter = Arc::new(TokenBucketRateLimit::new(64));
        let successes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (limiter, successes) = (limiter.clone(), successes.clone());
                scope.spawn(move || {
                    for _ in 0..64 {
                        if limiter.try_acquire(&record(tracing::Level::INFO, "t")) {
                            successes.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        });
        assert_eq!(
            successes.load(Ordering::Relaxed),
            64,
            "concurrent acquires must never oversell the budget"
        );
        assert_eq!(limiter.available(), 0);
    }

    #[tokio::test]
    async fn test_rate_limited_sink_flush_health_shutdown_forwarded() {
        let inner = Arc::new(LifecycleProbeSink {
            healthy: std::sync::atomic::AtomicBool::new(true),
            flushes: std::sync::atomic::AtomicUsize::new(0),
            shutdowns: std::sync::atomic::AtomicUsize::new(0),
        });
        let sink = RateLimitedSink::new(inner.clone() as Arc<dyn LogSink>, Arc::new(NoOpRateLimit));
        assert!(
            sink.is_healthy(),
            "inner healthy must surface through the wrapper"
        );
        inner
            .healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);
        assert!(
            !sink.is_healthy(),
            "inner unhealthy must surface through the wrapper"
        );
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
