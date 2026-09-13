// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Sink 中间件链（transform/filter 组合）。
//!
//! [`RecordMiddleware`] 在记录进入 sink 前做改写（transform）或丢弃
//! （filter）；[`MiddlewareChain`] 按注册顺序短路执行；[`MiddlewareSink`]
//! 是 [`LogSink`] 装饰器，把链条接入任意 sink（内置或动态注册的
//! 第三方 sink）。被丢弃的记录计入 `logs_dropped`。
//!
//! # Example
//! ```
//! use inklog::support::io::sink::middleware::{
//!     EnrichMiddleware, LevelFilterMiddleware, MiddlewareChain, MiddlewareSink,
//! };
//! use inklog::support::io::ConsoleSink;
//! use inklog::LogTemplate;
//! use std::sync::Arc;
//!
//! let chain = MiddlewareChain::new()
//!     .push(Arc::new(LevelFilterMiddleware::new("warn").expect("valid level")))
//!     .push(Arc::new(EnrichMiddleware::new("cluster", "prod-1")));
//! let inner: Arc<dyn inklog::LogSink> = Arc::new(ConsoleSink::new(
//!     Default::default(),
//!     LogTemplate::new("{timestamp} [{level}] {message}"),
//! ));
//! let sink: Arc<dyn inklog::LogSink> =
//!     Arc::new(MiddlewareSink::new(inner, chain));
//! ```

use std::sync::Arc;

use async_trait::async_trait;

use crate::support::io::LogSink;
use crate::support::query::level_rank;
use crate::{InklogError, LogRecord, Metrics};

/// 中间件对单条记录的裁决。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddlewareVerdict {
    /// 继续链条 / 写入 sink
    Continue,
    /// 丢弃该记录（链条短路，不写 sink）
    Drop,
}

/// 记录中间件端口：对象安全，`process` 同步执行（热路径无锁由实现方保证）。
pub trait RecordMiddleware: Send + Sync {
    /// 中间件名（诊断/指标标签）。
    fn name(&self) -> &str;

    /// 处理一条记录（可原地改写字段/消息）。
    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict;
}

/// 有序中间件链：任一环节 `Drop` 即短路。
#[derive(Default)]
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn RecordMiddleware>>,
}

impl std::fmt::Debug for MiddlewareChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareChain")
            .field(
                "middlewares",
                &self
                    .middlewares
                    .iter()
                    .map(|m| m.name())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加中间件（builder 风格）。
    pub fn push(mut self, middleware: Arc<dyn RecordMiddleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// 已注册的中间件数量。
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// 依序应用全部中间件；返回 `false` 表示记录被丢弃。
    pub fn apply(&self, record: &mut LogRecord) -> bool {
        for middleware in &self.middlewares {
            if middleware.process(record) == MiddlewareVerdict::Drop {
                return false;
            }
        }
        true
    }
}

/// filter 内置件：低于阈值的记录丢弃（阈值语义与采样器一致）。
#[derive(Debug, Clone)]
pub struct LevelFilterMiddleware {
    min_rank: u8,
}

impl LevelFilterMiddleware {
    /// `min_level`: trace/debug/info/warn/error/fatal（大小写不敏感）。
    ///
    /// # Errors
    ///
    /// 未知级别名返回 `InklogError::ConfigError`——静默构造会让过滤器
    /// 丢弃全部记录。
    pub fn new(min_level: &str) -> Result<Self, InklogError> {
        let rank = level_rank(min_level);
        if rank == u8::MAX {
            return Err(InklogError::ConfigError(format!(
                "unknown min level '{min_level}' for level-filter middleware; \
                 valid levels: trace/debug/info/warn/error/fatal"
            )));
        }
        Ok(Self { min_rank: rank })
    }
}

impl RecordMiddleware for LevelFilterMiddleware {
    fn name(&self) -> &str {
        "level-filter"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        let rank = level_rank(&record.level);
        if rank == u8::MAX || rank < self.min_rank {
            MiddlewareVerdict::Drop
        } else {
            MiddlewareVerdict::Continue
        }
    }
}

/// transform 内置件：为每条记录注入静态富化字段（如 cluster/env/tenant）。
#[derive(Debug, Clone)]
pub struct EnrichMiddleware {
    key: String,
    value: serde_json::Value,
}

impl EnrichMiddleware {
    pub fn new(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl RecordMiddleware for EnrichMiddleware {
    fn name(&self) -> &str {
        "enrich"
    }

    fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
        record.fields.insert(self.key.clone(), self.value.clone());
        MiddlewareVerdict::Continue
    }
}

/// Sink 中间件装饰器：链条裁决 `Continue` 才写内层 sink。
pub struct MiddlewareSink {
    inner: Arc<dyn LogSink>,
    chain: MiddlewareChain,
    metrics: Option<Arc<Metrics>>,
}

impl MiddlewareSink {
    pub fn new(inner: Arc<dyn LogSink>, chain: MiddlewareChain) -> Self {
        Self {
            inner,
            chain,
            metrics: None,
        }
    }

    /// 绑定指标（丢弃计入 `logs_dropped`）。
    pub fn with_metrics(mut self, metrics: Arc<Metrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// 已装配的链条。
    pub fn chain(&self) -> &MiddlewareChain {
        &self.chain
    }
}

#[async_trait]
impl LogSink for MiddlewareSink {
    async fn write(&self, record: &LogRecord) -> Result<(), InklogError> {
        // 空链快路：无中间件时零拷贝直通内层 sink
        if self.chain.is_empty() {
            return self.inner.write(record).await;
        }
        let mut record = record.clone();
        if !self.chain.apply(&mut record) {
            if let Some(ref metrics) = self.metrics {
                metrics.inc_logs_dropped();
            }
            return Ok(());
        }
        self.inner.write(&record).await
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
    use tracing::Level;

    fn record(level: Level, message: &str) -> LogRecord {
        LogRecord::new(level, "mw::test".to_string(), message.to_string())
    }

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

    /// 丢弃包含特定关键词的记录（自定义 filter 中间件样例）。
    struct KeywordDropMiddleware {
        keyword: String,
    }

    impl RecordMiddleware for KeywordDropMiddleware {
        fn name(&self) -> &str {
            "keyword-drop"
        }
        fn process(&self, record: &mut LogRecord) -> MiddlewareVerdict {
            if record.message.contains(&self.keyword) {
                MiddlewareVerdict::Drop
            } else {
                MiddlewareVerdict::Continue
            }
        }
    }

    #[test]
    fn test_level_filter_middleware() {
        let mw = LevelFilterMiddleware::new("warn").expect("valid level");
        assert_eq!(
            mw.process(&mut record(Level::ERROR, "x")),
            MiddlewareVerdict::Continue
        );
        assert_eq!(
            mw.process(&mut record(Level::WARN, "x")),
            MiddlewareVerdict::Continue
        );
        assert_eq!(
            mw.process(&mut record(Level::INFO, "x")),
            MiddlewareVerdict::Drop
        );
    }

    #[test]
    fn test_level_filter_middleware_rejects_unknown_level() {
        // 未知级别必须构造期报错：静默构造会丢弃全部记录
        assert!(LevelFilterMiddleware::new("foobar").is_err());
        assert!(LevelFilterMiddleware::new("").is_err());
        assert!(LevelFilterMiddleware::new("WARN").is_ok());
    }

    #[test]
    fn test_chain_applies_in_order_and_short_circuits() {
        let chain = MiddlewareChain::new()
            .push(Arc::new(KeywordDropMiddleware {
                keyword: "secret".to_string(),
            }))
            .push(Arc::new(EnrichMiddleware::new("cluster", "prod-1")));

        // 命中丢弃：短路（不再 enrich）
        let mut dropped = record(Level::INFO, "has secret inside");
        assert!(!chain.apply(&mut dropped));
        assert!(
            !dropped.fields.contains_key("cluster"),
            "drop must short-circuit"
        );

        // 通过：enrich 生效
        let mut kept = record(Level::INFO, "plain message");
        assert!(chain.apply(&mut kept));
        assert_eq!(kept.fields.get("cluster").unwrap(), "prod-1");
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_middleware_sink_filters_and_enriches() {
        let inner = Arc::new(CollectingSink {
            messages: Mutex::new(Vec::new()),
        });
        let metrics = Arc::new(Metrics::new());
        let sink = Arc::new(
            MiddlewareSink::new(
                inner.clone() as Arc<dyn LogSink>,
                MiddlewareChain::new()
                    .push(Arc::new(
                        LevelFilterMiddleware::new("warn").expect("valid level"),
                    ))
                    .push(Arc::new(EnrichMiddleware::new("tenant", "acme"))),
            )
            .with_metrics(metrics.clone()),
        );

        sink.write(&record(Level::ERROR, "keep me")).await.unwrap();
        sink.write(&record(Level::INFO, "drop me")).await.unwrap();

        let messages = inner.messages.lock();
        assert_eq!(
            messages.len(),
            1,
            "filtered record must not reach inner sink"
        );
        drop(messages);
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "filtered record counts as dropped"
        );
    }

    #[tokio::test]
    async fn test_middleware_sink_health_and_shutdown_forwarded() {
        struct HealthSink {
            healthy: std::sync::atomic::AtomicBool,
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
                self.healthy.load(std::sync::atomic::Ordering::Relaxed)
            }
            async fn shutdown(&self) -> Result<(), InklogError> {
                Ok(())
            }
        }
        let inner = Arc::new(HealthSink {
            healthy: std::sync::atomic::AtomicBool::new(true),
        });
        let sink = MiddlewareSink::new(inner.clone() as Arc<dyn LogSink>, MiddlewareChain::new());
        assert!(sink.is_healthy());
        inner
            .healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);
        assert!(!sink.is_healthy(), "health must be forwarded");
    }
}
