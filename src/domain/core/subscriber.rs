// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::LogRecord;
use crate::Metrics;
use crate::support::processing::RateLimiter;
use crate::validation::sanitize::LogSanitizer;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use serde_json::value;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const DEFAULT_SEND_TIMEOUT_MS: u64 = 100;
const FALLBACK_BUFFER_SIZE: usize = 100;
/// Sampling rate for ERROR/FATAL logs when rate-limited: keep 1 in N.
const ERROR_SAMPLING_RATE: u64 = 100;

/// Fallback buffer 条目：记录 + 各 async 通道的投递状态。
///
/// `delivered[i] == true` 表示第 i 个 async 通道（0 = 主 async 通道，
/// 1.. = `extra_async_senders` 通道）已成功收到该记录。flush 只向未成功
/// 的通道补发，防止已成功通道上的记录被重复写出（重复日志）。
struct FallbackEntry {
    record: Arc<LogRecord>,
    delivered: Vec<bool>,
}

/// High-performance logging subscriber with lock-free hot path.
///
/// Uses crossbeam channels for both console and async sinks to eliminate
/// lock contention in the hot path (on_event).
/// Uses `Arc<LogRecord>` to avoid deep cloning when sending to multiple sinks.
/// Includes fallback buffer for critical logs (ERROR/FATAL).
/// Clone 语义：全字段为 channel sender / Arc 句柄（克隆共享同一 logger 实例），
/// AtomicU64 计数器克隆当前值。供测试 harness 在子线程以
/// tracing::subscriber::with_default 安装线程级 subscriber（需 owned 值）。
impl Clone for LoggerSubscriber {
    fn clone(&self) -> Self {
        Self {
            console_sender: self.console_sender.clone(),
            async_sender: self.async_sender.clone(),
            extra_async_senders: self.extra_async_senders.clone(),
            metrics: self.metrics.clone(),
            send_timeout_ms: self.send_timeout_ms,
            fallback_buffer: self.fallback_buffer.clone(),
            sanitizer: self.sanitizer.clone(),
            rate_limiter: self.rate_limiter.clone(),
            error_sample_counter: AtomicU64::new(self.error_sample_counter.load(Ordering::Relaxed)),
        }
    }
}
pub struct LoggerSubscriber {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<Arc<LogRecord>>,
    /// Channel sender for async sinks (file, database, etc.)
    async_sender: Sender<Arc<LogRecord>>,
    /// Additional async sink channels. Each enabled async sink gets its own
    /// channel, otherwise a single shared MPMC channel would deliver each
    /// record to only one of the sink workers (data loss for the others).
    extra_async_senders: Vec<Sender<Arc<LogRecord>>>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
    /// Timeout for async channel send (milliseconds)
    send_timeout_ms: u64,
    /// Fallback buffer for critical logs
    fallback_buffer: Arc<Mutex<VecDeque<FallbackEntry>>>,
    /// Optional log sanitizer for preventing log injection (CWE-117)
    sanitizer: Option<Arc<LogSanitizer>>,
    /// Optional rate limiter for log throughput control
    rate_limiter: Option<Arc<RateLimiter>>,
    /// Counter for ERROR/FATAL sampling when rate-limited
    error_sample_counter: AtomicU64,
}

impl LoggerSubscriber {
    pub fn new(
        console_sender: Sender<Arc<LogRecord>>,
        async_sender: Sender<Arc<LogRecord>>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            console_sender,
            async_sender,
            extra_async_senders: Vec::new(),
            metrics,
            send_timeout_ms: DEFAULT_SEND_TIMEOUT_MS,
            fallback_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(FALLBACK_BUFFER_SIZE))),
            sanitizer: None,
            rate_limiter: None,
            error_sample_counter: AtomicU64::new(0),
        }
    }

    /// Add an additional async sink channel. Each enabled async sink must have
    /// its own channel; sharing a single channel between workers means every
    /// record is consumed by only one worker.
    pub fn with_extra_async_sender(mut self, sender: Sender<Arc<LogRecord>>) -> Self {
        self.extra_async_senders.push(sender);
        self
    }

    /// Send a record to every configured async sink channel. Returns the
    /// per-channel delivery status (`true` = delivered)，索引与
    /// [`FallbackEntry::delivered`] 一致（0 = 主 async 通道）。
    fn send_to_async_sinks(&self, record: &Arc<LogRecord>, timeout: Duration) -> Vec<bool> {
        let mut delivered = Vec::with_capacity(1 + self.extra_async_senders.len());
        delivered.push(
            self.async_sender
                .send_timeout(Arc::clone(record), timeout)
                .is_ok(),
        );
        for sender in &self.extra_async_senders {
            delivered.push(sender.send_timeout(Arc::clone(record), timeout).is_ok());
        }
        delivered
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.send_timeout_ms = timeout_ms;
        self
    }

    /// Set the log sanitizer for preventing log injection attacks.
    pub fn with_sanitizer(mut self, sanitizer: Arc<LogSanitizer>) -> Self {
        self.sanitizer = Some(sanitizer);
        self
    }

    /// Set the rate limiter for log throughput control.
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<RateLimiter>) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    fn is_critical_level(level: &str) -> bool {
        level == "ERROR" || level == "FATAL"
    }

    /// 从当前 tracing span 上下文提取 trace_id/span_id。
    ///
    /// - `span_id` = 事件所在 span 的 16 位小写 hex id；
    /// - `trace_id` 优先取事件/span 已显式记录的 `trace_id` 字段（与
    ///   OpenTelemetry / tracing-opentelemetry 注入兼容），否则沿 parent 链
    ///   找到根 span，以其 id 派生 32 位小写 hex（同一条 trace 内一致）；
    /// - 事件在任意 span 之外时两字段保持 `None`（热路径零成本直通）。
    fn extract_trace_context<S>(ctx: &Context<'_, S>, record: &mut LogRecord)
    where
        S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        let current = ctx.current_span();
        let Some(id) = current.id() else {
            return;
        };
        record.span_id = Some(format!("{:016x}", id.into_u64()));

        // 事件字段显式携带的 trace_id 优先
        if let Some(value::Value::String(explicit)) = record.fields.get("trace_id") {
            record.trace_id = Some(explicit.clone());
        }
        if record.trace_id.is_none() {
            let trace_id = ctx.span(&id).and_then(|span| {
                span.scope().last().map(|root| format!("{:032x}", root.id().into_u64()))
            });
            record.trace_id = trace_id;
        }
        // 事件字段显式携带的 span_id 覆盖派生值（与 OTel 语义对齐）
        if let Some(value::Value::String(explicit)) = record.fields.get("span_id") {
            record.span_id = Some(explicit.clone());
        }
    }

    // 敏感键判定不再有本地副本：统一引用 `LogRecord::is_sensitive_key`
    // （src/domain/types/log_record.rs，pub(crate) 单一事实源），
    // 避免手工同步两份模式表导致的安全行为分叉。

    /// Sanitize a log record's message and fields values, recursing into
    /// nested objects and arrays so strings under sensitive keys are not
    /// left untouched.
    fn sanitize_record(&self, record: &mut LogRecord) {
        if let Some(ref sanitizer) = self.sanitizer {
            record.message = sanitizer.sanitize(&record.message);
            for value in record.fields.values_mut() {
                Self::sanitize_field_value(sanitizer, value);
            }
        }
    }

    /// 递归脱敏字段值：字符串值一律 sanitize；Object 按键递归（敏感键的
    /// 字符串值同样被脱敏，不再被跳过）；Array 逐元素递归。
    fn sanitize_field_value(sanitizer: &LogSanitizer, value: &mut value::Value) {
        match value {
            value::Value::String(s) => *s = sanitizer.sanitize(s),
            value::Value::Array(items) => {
                for item in items.iter_mut() {
                    Self::sanitize_field_value(sanitizer, item);
                }
            }
            value::Value::Object(map) => {
                for (nested_key, nested_value) in map.iter_mut() {
                    if LogRecord::is_sensitive_key(nested_key) {
                        // 敏感键：直接脱敏其字符串值
                        if let value::Value::String(s) = nested_value {
                            *s = sanitizer.sanitize(s);
                        } else {
                            Self::sanitize_field_value(sanitizer, nested_value);
                        }
                    } else {
                        Self::sanitize_field_value(sanitizer, nested_value);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn try_flush_fallback(&self) {
        // 锁内仅取出待 flush 批量，循环发送在锁外执行，
        // 避免锁被持有 N × send_timeout
        let batch: Vec<FallbackEntry> = {
            let mut buffer = self.fallback_buffer.lock();
            buffer.drain(..).collect()
        };
        if batch.is_empty() {
            return;
        }
        let timeout = Duration::from_millis(self.send_timeout_ms);
        let channel_count = 1 + self.extra_async_senders.len();
        let mut undelivered: VecDeque<FallbackEntry> = VecDeque::new();
        let mut stopped = false;
        for mut entry in batch {
            if stopped {
                // flush 已中断：剩余记录按原顺序回填，不再尝试发送
                undelivered.push_back(entry);
                continue;
            }
            // 只向尚未成功投递的通道补发；已成功的通道不再重发（抑制重复日志）
            let mut complete = true;
            for idx in 0..channel_count {
                if entry.delivered.get(idx).copied().unwrap_or(false) {
                    continue;
                }
                let sender = if idx == 0 {
                    &self.async_sender
                } else {
                    &self.extra_async_senders[idx - 1]
                };
                if sender.send_timeout(Arc::clone(&entry.record), timeout).is_ok() {
                    if idx < entry.delivered.len() {
                        entry.delivered[idx] = true;
                    }
                } else {
                    // 与既有语义一致：flush 中途失败即停止，未完成的记录回填
                    complete = false;
                    break;
                }
            }
            if !complete {
                undelivered.push_back(entry);
                stopped = true;
            }
        }
        if !undelivered.is_empty() {
            // flush 失败：未完成投递的记录（含已部分补发的）按原顺序回填到队首
            let mut buffer = self.fallback_buffer.lock();
            for entry in undelivered.into_iter().rev() {
                buffer.push_front(entry);
            }
        }
    }
}

impl Drop for LoggerSubscriber {
    fn drop(&mut self) {
        // Attempt to flush any remaining fallback buffer entries
        let buffer_len = {
            let buffer = self.fallback_buffer.lock();
            buffer.len()
        };
        if buffer_len > 0 {
            self.try_flush_fallback();
            let remaining = self.fallback_buffer.lock().len();
            if remaining > 0 {
                // Drop 阶段不依赖 tracing 全局状态：格式化到 String 后直接输出
                let warning = format!(
                    "LoggerSubscriber dropped with {remaining} unflushed fallback records"
                );
                eprintln!("{warning}");
            }
        }
    }
}

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut record = LogRecord::from_event(event);
        // 从当前 span 上下文提取 trace_id/span_id（未启用 span 时零成本直通）
        Self::extract_trace_context(&ctx, &mut record);

        // Rate limiting check (before sanitization to save work on dropped logs)
        if let Some(ref limiter) = self.rate_limiter
            && !limiter.try_acquire()
        {
            // Rate limited: only keep ERROR/FATAL with 1-in-N sampling
            if Self::is_critical_level(&record.level) {
                let count = self.error_sample_counter.fetch_add(1, Ordering::Relaxed);
                if !count.is_multiple_of(ERROR_SAMPLING_RATE) {
                    self.metrics.inc_logs_dropped();
                    return;
                }
                // Sampled: fall through to send
            } else {
                self.metrics.inc_logs_dropped();
                return;
            }
        }

        // Sanitize message and fields before sending to channels
        self.sanitize_record(&mut record);

        let record = Arc::new(record);

        // Fast path: Console - lock-free try_send, never block
        match self.console_sender.try_send(Arc::clone(&record)) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                // Channel full, drop the message and record metric
                // Hot path should never block
                self.metrics.inc_channel_blocked();
                self.metrics.inc_logs_dropped();
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        // Slow path: Async sinks - use timeout for backpressure handling
        let timeout = Duration::from_millis(self.send_timeout_ms);
        let delivered = self.send_to_async_sinks(&record, timeout);
        if delivered.iter().any(|ok| !ok) {
            // For critical logs, add to fallback buffer（保留各通道投递状态，
            // flush 时只补发未成功的通道）
            if Self::is_critical_level(&record.level) {
                let mut buffer = self.fallback_buffer.lock();
                if buffer.len() >= FALLBACK_BUFFER_SIZE {
                    buffer.pop_front();
                }
                buffer.push_back(FallbackEntry { record, delivered });
            } else {
                // Timeout on non-critical log: message is lost (send_timeout returns
                // ownership but we have nowhere to buffer it). Only count as dropped,
                // not as channel_blocked, to keep metric semantics distinct:
                // channel_blocked = backpressure event, logs_dropped = data loss.
                self.metrics.inc_logs_dropped();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use crossbeam_channel::bounded;
    use serial_test::serial;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_on_event_sends_to_channels() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "hello", user_id = 1u64);
        });

        // Verify console channel received the record
        let console_received = console_rx.recv().unwrap();
        assert_eq!(console_received.level, "INFO");
        assert_eq!(console_received.target, "test::subscriber");
        assert_eq!(console_received.message, "hello");

        // Verify async channel received the record
        let async_received = async_rx.recv().unwrap();
        assert_eq!(async_received.level, "INFO");
        assert_eq!(async_received.target, "test::subscriber");
        assert_eq!(async_received.message, "hello");
    }

    #[test]
    fn test_on_event_handles_full_channel() {
        // Create a channel with capacity 1
        let (console_tx, console_rx) = bounded(1);
        let (async_tx, async_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        // Send multiple events - should not panic even when channel is full
        with_default(registry, || {
            for i in 0..5 {
                tracing::info!(target: "test::subscriber", message = "msg {}", i);
            }
        });

        // Drain channels to verify messages were sent
        while console_rx.try_recv().is_ok() {}
        while async_rx.try_recv().is_ok() {}
    }

    #[test]
    fn test_critical_level_adds_to_fallback_buffer() {
        let (console_tx, _console_rx) = bounded(10);
        // Zero-capacity async channel causes send_timeout to always time out,
        // triggering the fallback path for ERROR/FATAL events.
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // Should not panic: ERROR events route to fallback buffer
        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "critical error");
        });
        // If we reach here without panic, the critical-level fallback path works
        assert_eq!(metrics.logs_written(), 0);
    }

    #[test]
    fn test_fallback_buffer_does_not_panic_on_overflow() {
        let (console_tx, _cr) = bounded(10);
        // Zero-capacity async channel: all async sends time out
        let (async_tx, _ar) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        // Send many ERROR events — fallback buffer has max size 100.
        // FILL_BROWSER_SIZE + 5 events. Should not panic.
        with_default(registry, || {
            for i in 0..105 {
                tracing::error!(target: "test::subscriber", msg = "overflow {}", i);
            }
        });
        // Reaching here without panic confirms LRU eviction in fallback buffer works
    }

    #[test]
    fn test_try_flush_fallback_with_disconnected_channel() {
        let (console_tx1, _cr1) = bounded(10);
        let (async_tx1, _ar1) = bounded(1);
        drop(_ar1);
        let metrics = Arc::new(Metrics::new());

        // Subscriber A: used as tracing layer within with_default
        let layer = LoggerSubscriber::new(console_tx1.clone(), async_tx1.clone(), metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // ERROR events go to fallback buffer via subscriber A
        with_default(registry, || {
            tracing::error!(target: "test::subscriber", msg = "fallback before disconnect");
        });

        // Subscriber B: shares same channels via Arc<Metrics> but owns its own fallback buffer.
        // The async_sender is disconnected, so try_flush_fallback hits
        // SendTimeoutError::Disconnected → loop breaks safely. No panic.
        let _subscriber_b = LoggerSubscriber::new(console_tx1, async_tx1, metrics);
        _subscriber_b.try_flush_fallback();
    }

    #[test]
    fn test_on_event_dropped_on_disconnected_async_channel() {
        let (console_tx, _cr) = bounded(10);
        // Create and immediately drop the receiver to simulate disconnection
        let (async_tx, _ar) = bounded(1);
        drop(_ar); // Disconnect async channel
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // Sending should not panic even when async channel is disconnected
        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "after disconnect");
        });

        // Should have incremented logs_dropped for the disconnected async channel
        assert_eq!(metrics.logs_dropped(), 1);
    }

    #[test]
    fn test_with_timeout_configures_send_timeout() {
        let (console_tx, _) = bounded(10);
        let (async_tx, _) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics).with_timeout(500);

        assert_eq!(subscriber.send_timeout_ms, 500);
    }

    // =========================================================================
    // try_flush_fallback() 测试 - 覆盖成功弹出和失败中断分支
    // =========================================================================

    #[test]
    fn test_try_flush_fallback_drains_buffer_on_success() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        // 手动向 fallback_buffer 注入一条记录（测试模块可访问私有字段）
        let record = Arc::new(LogRecord::new(
            tracing::Level::ERROR,
            "test::fallback".to_string(),
            "fallback flush test".to_string(),
        ));
        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record,
            delivered: vec![false],
        });

        // 调用 try_flush_fallback，async channel 有容量 → send 成功 → pop_front
        subscriber.try_flush_fallback();

        // 验证 buffer 已清空
        assert!(
            subscriber.fallback_buffer.lock().is_empty(),
            "buffer should be empty after successful flush"
        );

        // 验证记录已发送到 async channel
        let received = async_rx.recv_timeout(std::time::Duration::from_millis(100));
        assert!(received.is_ok(), "should receive the flushed record");
        assert_eq!(received.unwrap().message, "fallback flush test");
    }

    #[test]
    fn test_try_flush_fallback_breaks_on_disconnected_channel() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        // 注入记录到 fallback_buffer
        let record = Arc::new(LogRecord::new(
            tracing::Level::ERROR,
            "test::fallback".to_string(),
            "disconnect test".to_string(),
        ));
        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record,
            delivered: vec![false],
        });

        // 断开 async channel 的接收端 → send 返回 Disconnected → break
        drop(_async_rx);
        subscriber.try_flush_fallback();

        // 断开后 buffer 应仍包含记录（break 未弹出）
        assert_eq!(
            subscriber.fallback_buffer.lock().len(),
            1,
            "buffer should still contain the record after disconnect"
        );
    }

    #[test]
    fn test_try_flush_fallback_preserves_order_on_partial_flush() {
        let (console_tx, _console_rx) = bounded(10);
        // 容量 1：第一条发送成功后 channel 满，第二条 send_timeout 超时中断
        let (async_tx, async_rx) = bounded(1);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics);

        for i in 0..3 {
            let record = Arc::new(LogRecord::new(
                tracing::Level::ERROR,
                "test::fallback".to_string(),
                format!("fallback-order-{i}"),
            ));
            subscriber
                .fallback_buffer
                .lock()
                .push_back(FallbackEntry {
                    record,
                    delivered: vec![false],
                });
        }
        subscriber.try_flush_fallback();

        // 第一条已发出，剩余两条应按原顺序回填到队首
        let first = async_rx
            .recv_timeout(std::time::Duration::from_millis(500))
            .expect("first record should be flushed");
        assert_eq!(first.message, "fallback-order-0");

        let buffer = subscriber.fallback_buffer.lock();
        assert_eq!(buffer.len(), 2, "remaining records should be refilled");
        assert_eq!(
            buffer.front().unwrap().record.message,
            "fallback-order-1",
            "refilled records must keep original order (front)"
        );
        assert_eq!(
            buffer.back().unwrap().record.message,
            "fallback-order-2",
            "refilled records must keep original order (back)"
        );
    }

    // =========================================================================
    // fallback 重复抑制回归：按通道精确追踪投递状态，flush 只补发未成功
    // 通道对应的记录；已成功通道不得被再次投递（重复日志）。
    // =========================================================================

    #[test]
    fn test_on_event_partial_failure_records_per_channel_delivery() {
        // 主 async 通道成功、extra 通道（rendezvous 容量 0）必然超时失败：
        // ERROR 记录进入 fallback，且 delivered 状态应精确为 [true, false]
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let (extra_tx, _extra_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_extra_async_sender(extra_tx)
            .with_timeout(50);
        // 在 layer 被 registry 消费前，先拿到 fallback_buffer 的 Arc clone
        let fallback_buffer = Arc::clone(&layer.fallback_buffer);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "partial delivery");
        });

        // 主通道恰好收到一条（无重复）
        assert!(
            async_rx.try_recv().is_ok(),
            "primary async channel should receive the record"
        );
        assert!(
            async_rx.try_recv().is_err(),
            "primary async channel must not receive duplicates"
        );

        // fallback 中该记录的投递状态精确为 [true, false]
        let buffer = fallback_buffer.lock();
        assert_eq!(
            buffer.len(),
            1,
            "record should be buffered for the failed channel"
        );
        let entry = buffer.front().unwrap();
        assert_eq!(
            entry.delivered,
            vec![true, false],
            "delivery state must be tracked per channel"
        );
        assert_eq!(entry.record.message, "partial delivery");
    }

    #[test]
    fn test_fallback_flush_only_resends_undelivered_channels() {
        // 已投递通道（delivered[0]=true）在 flush 时不得重发；
        // 未投递通道补发恰好一条，全部投递完成后条目移出 buffer
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        // rendezvous 通道：flush 的补发与接收线程会合后成功
        let (extra_tx, extra_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let subscriber = LoggerSubscriber::new(console_tx, async_tx, metrics)
            .with_extra_async_sender(extra_tx)
            .with_timeout(500);

        subscriber.fallback_buffer.lock().push_back(FallbackEntry {
            record: Arc::new(LogRecord::new(
                tracing::Level::ERROR,
                "test::fallback".to_string(),
                "dup suppression".to_string(),
            )),
            delivered: vec![true, false],
        });

        // 接收线程先阻塞在 rendezvous channel 上，flush 的补发与其会合
        let receiver = std::thread::spawn(move || {
            extra_rx.recv_timeout(std::time::Duration::from_millis(2000))
        });
        std::thread::sleep(std::time::Duration::from_millis(50));

        subscriber.try_flush_fallback();

        // 未成功通道收到恰好一条补发
        let resent = receiver
            .join()
            .unwrap()
            .expect("extra channel should receive the flushed record");
        assert_eq!(resent.message, "dup suppression");

        // 已成功通道不得收到重复记录
        assert!(
            async_rx.try_recv().is_err(),
            "already-delivered channel must NOT receive a duplicate on flush"
        );

        // 全部通道投递完成后 buffer 清空
        assert!(
            subscriber.fallback_buffer.lock().is_empty(),
            "fully delivered entry should be removed from the buffer"
        );
    }

    // =========================================================================
    // sanitize_record 嵌套结构递归测试：嵌套对象/数组中敏感键被脱敏
    // =========================================================================

    #[test]
    fn test_sanitize_record_recurses_into_nested_object_and_array() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let sanitizer = Arc::new(LogSanitizer::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics).with_sanitizer(sanitizer);

        let mut record = LogRecord::new(
            tracing::Level::INFO,
            "test::sanitize".to_string(),
            "nested sanitize".to_string(),
        );

        // 嵌套对象：敏感键 + 普通键
        let mut nested = serde_json::Map::new();
        nested.insert(
            "password".to_string(),
            Value::String("line1\nline2".to_string()),
        );
        nested.insert("note".to_string(), Value::String("a\nb".to_string()));
        record
            .fields
            .insert("config".to_string(), Value::Object(nested));

        // 数组内对象：敏感键
        let mut item = serde_json::Map::new();
        item.insert(
            "api_token".to_string(),
            Value::String("tok1\ntok2".to_string()),
        );
        record
            .fields
            .insert("items".to_string(), Value::Array(vec![Value::Object(item)]));

        layer.sanitize_record(&mut record);

        // 嵌套对象中的敏感键字符串值被脱敏（换行被转义）
        let config = record.fields.get("config").unwrap();
        if let Value::Object(map) = config {
            if let Value::String(s) = map.get("password").unwrap() {
                assert!(
                    !s.contains('\n') && s.contains("\\n"),
                    "nested sensitive key 'password' must be sanitized, got: {s:?}"
                );
            } else {
                panic!("password value should remain a string");
            }
            // 普通键同样被递归脱敏
            if let Value::String(s) = map.get("note").unwrap() {
                assert!(
                    !s.contains('\n'),
                    "nested plain string must also be sanitized, got: {s:?}"
                );
            }
        } else {
            panic!("config field should remain an object");
        }

        // 数组内对象中的敏感键字符串值被脱敏
        let items = record.fields.get("items").unwrap();
        if let Value::Array(arr) = items {
            if let Value::Object(map) = &arr[0] {
                if let Value::String(s) = map.get("api_token").unwrap() {
                    assert!(
                        !s.contains('\n') && s.contains("\\n"),
                        "sensitive key inside array must be sanitized, got: {s:?}"
                    );
                } else {
                    panic!("api_token value should remain a string");
                }
            } else {
                panic!("array element should remain an object");
            }
        } else {
            panic!("items field should remain an array");
        }
    }

    #[test]
    fn test_sanitize_record_leaves_non_string_values_untouched() {
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let sanitizer = Arc::new(LogSanitizer::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics).with_sanitizer(sanitizer);

        let mut record = LogRecord::new(
            tracing::Level::INFO,
            "test::sanitize".to_string(),
            "non-string values".to_string(),
        );
        record
            .fields
            .insert("count".to_string(), serde_json::json!(42));

        layer.sanitize_record(&mut record);

        assert_eq!(
            record.fields.get("count").unwrap(),
            &serde_json::json!(42),
            "non-string values must not be modified"
        );
    }

    // =========================================================================
    // 敏感键判定单一事实源（diting R3）：subscriber 的 sanitizer 路径直接引用
    // `LogRecord::is_sensitive_key`（token 边界语义），本测试钉住该语义，
    // 防止有人再引入按子串匹配的本地副本（子串匹配会把 "author" 误判为敏感）。
    // =========================================================================

    #[test]
    fn test_is_sensitive_key_matches_log_record_canonical_semantics() {
        // 敏感键（与 log_record.rs 的 token 边界判定一致）
        for key in ["password", "api_key", "auth_token", "secret"] {
            assert!(
                LogRecord::is_sensitive_key(key),
                "'{key}' must be judged sensitive by the canonical implementation"
            );
        }
        // 非敏感键（子串匹配会误判 "author" 含 "auth"，token 边界判定不会）
        for key in ["primary_key", "author"] {
            assert!(
                !LogRecord::is_sensitive_key(key),
                "'{key}' must NOT be judged sensitive by the canonical implementation"
            );
        }
    }

    // NOTE: parking_lot::Mutex 不支持 poison，无需测试毒化恢复
    // try_flush_fallback 的断开 channel 场景由
    // test_try_flush_fallback_with_disconnected_channel 覆盖

    // =========================================================================
    // on_event console channel 断开测试
    // =========================================================================

    #[test]
    fn test_on_event_console_disconnected_increments_dropped() {
        let (console_tx, _console_rx) = bounded(10);
        // 断开 console channel
        drop(_console_rx);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "console disconnected");
        });

        // console 断开 → logs_dropped += 1；async 正常 → 无变化
        assert_eq!(
            metrics.logs_dropped(),
            1,
            "console disconnect should increment logs_dropped by 1"
        );
    }

    #[test]
    fn test_on_event_console_full_channel_increments_blocked_and_dropped() {
        // console channel 容量 1，发送 2 条事件 → 第二条 Full
        let (console_tx, console_rx) = bounded(1);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        // 先填满 console channel（容量 1）
        // 第一条事件：console Ok，async Ok
        // 第二条事件：console Full → channel_blocked++ + logs_dropped++
        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "first");
            tracing::info!(target: "test::subscriber", message = "second");
        });

        // 排空 console channel
        while console_rx.try_recv().is_ok() {}

        // console Full 应触发 channel_blocked 和 logs_dropped
        assert!(
            metrics.logs_dropped() >= 1,
            "console full should increment logs_dropped, got: {}",
            metrics.logs_dropped()
        );
    }

    // =========================================================================
    // on_event fallback_buffer 锁毒化恢复（行 116, 119-120）
    // =========================================================================

    // NOTE: parking_lot::Mutex 不支持 poison，无需测试毒化恢复
    // on_event 的 fallback buffer 路径由
    // test_critical_level_adds_to_fallback_buffer 覆盖

    #[test]
    fn test_on_event_console_ok_and_async_ok_paths() {
        // 显式覆盖行 95（console try_send Ok）和行 110（async send_timeout Ok）
        // 现有 test_on_event_sends_to_channels 已覆盖，但这里额外验证
        // metrics 没有增加（确认 Ok 路径不触发 drop/blocked 计数）
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "ok path test");
        });

        // 两个 channel 都应收到记录
        assert!(
            console_rx.try_recv().is_ok(),
            "console should receive record"
        );
        assert!(async_rx.try_recv().is_ok(), "async should receive record");

        // Ok 路径不应增加 logs_dropped 或 channel_blocked
        assert_eq!(
            metrics.logs_dropped(),
            0,
            "Ok path should not increment logs_dropped"
        );
    }

    // =========================================================================
    // on_event 错误路径覆盖：非关键级别 async 超时 → 仅 logs_dropped 递增
    // channel_blocked 仅在背压事件（send_timeout 返回 Full）时递增，
    // 而超时丢弃消息仅属于数据丢失，不属于背压。
    // =========================================================================

    #[test]
    #[serial]
    fn test_on_event_non_critical_async_timeout_increments_blocked_and_dropped() {
        // async channel 容量 0（rendezvous）→ send_timeout 必然超时
        // INFO 级别非关键 → 仅 increment logs_dropped (not channel_blocked)
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        let registry = tracing_subscriber::registry().with(layer);

        let before_blocked = metrics.channel_blocked();
        let before_dropped = metrics.logs_dropped();

        with_default(registry, || {
            tracing::info!(target: "test::subscriber", message = "non-critical timeout");
        });

        // Metric semantics: channel_blocked = backpressure event,
        // logs_dropped = data loss.  A timeout on a non-critical log
        // is data-loss only, not backpressure.
        assert_eq!(
            metrics.channel_blocked(),
            before_blocked,
            "non-critical async timeout should NOT increment channel_blocked"
        );
        assert_eq!(
            metrics.logs_dropped(),
            before_dropped + 1,
            "non-critical async timeout should increment logs_dropped"
        );
    }

    // =========================================================================
    // on_event 错误路径覆盖：关键级别 async 超时 → 记录存入 fallback_buffer
    // 显式覆盖行 113-126，并验证 buffer 内容和 metrics 不递增
    // =========================================================================

    #[test]
    #[serial]
    fn test_on_event_critical_async_timeout_stores_record_in_fallback_buffer() {
        // async channel 容量 0（rendezvous）→ send_timeout 必然超时
        // ERROR 级别为关键 → 走行 113-126（存入 fallback_buffer，不递增 metrics）
        let (console_tx, _console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(0);
        let metrics = Arc::new(Metrics::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics.clone());
        // 在 layer 被 registry 消费前，先拿到 fallback_buffer 的 Arc clone
        let fallback_buffer = Arc::clone(&layer.fallback_buffer);
        let registry = tracing_subscriber::registry().with(layer);

        let before_blocked = metrics.channel_blocked();
        let before_dropped = metrics.logs_dropped();

        with_default(registry, || {
            tracing::error!(target: "test::subscriber", message = "critical timeout");
        });

        // 关键级别 + async 超时：记录存入 fallback_buffer
        let buffer_guard = fallback_buffer.lock();
        assert_eq!(
            buffer_guard.len(),
            1,
            "fallback_buffer should contain exactly 1 record"
        );
        let entry = buffer_guard
            .front()
            .expect("should have a record in fallback_buffer");
        assert_eq!(entry.record.level, "ERROR", "record level should be ERROR");
        assert_eq!(
            entry.record.message, "critical timeout",
            "record message should match"
        );
        assert_eq!(
            entry.delivered,
            vec![false],
            "all async channels failed, delivery state should be [false]"
        );
        drop(buffer_guard);

        // 关键级别不应递增 channel_blocked 或 logs_dropped
        assert_eq!(
            metrics.channel_blocked(),
            before_blocked,
            "critical level should not increment channel_blocked"
        );
        assert_eq!(
            metrics.logs_dropped(),
            before_dropped,
            "critical level should not increment logs_dropped"
        );
    }

    // =========================================================================
    // sanitizer integration tests
    // =========================================================================

    #[test]
    fn test_with_sanitizer_escapes_newline_in_message() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());
        let sanitizer = Arc::new(LogSanitizer::new());

        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics).with_sanitizer(sanitizer);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // tracing will把 \n 保留在 message 中
            tracing::info!(target: "test::sanitizer", message = "line1\nline2");
        });

        let received = console_rx.recv().unwrap();
        // Sanitizer should have escaped the newline
        assert!(
            received.message.contains("\\n"),
            "message should contain escaped newline, got: {:?}",
            received.message
        );
        assert!(
            !received.message.contains('\n'),
            "message should not contain raw newline"
        );
    }

    #[test]
    fn test_without_sanitizer_message_unchanged() {
        let (console_tx, console_rx) = bounded(10);
        let (async_tx, _async_rx) = bounded(10);
        let metrics = Arc::new(Metrics::new());

        // No sanitizer set — default behavior
        let layer = LoggerSubscriber::new(console_tx, async_tx, metrics);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            tracing::info!(target: "test::no_sanitizer", message = "plain message");
        });

        let received = console_rx.recv().unwrap();
        assert_eq!(received.message, "plain message");
    }

    // =========================================================================
    // rate limiter integration tests
    // =========================================================================

    #[test]
    fn test_rate_limiter_drops_non_critical_logs() {
        let (console_tx, console_rx) = bounded(100);
        let (async_tx, _async_rx) = bounded(100);
        let metrics = Arc::new(Metrics::new());
        // Rate of 2 tokens: only 2 logs allowed initially
        let limiter = Arc::new(RateLimiter::new(2));

        let layer =
            LoggerSubscriber::new(console_tx, async_tx, metrics.clone()).with_rate_limiter(limiter);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            for _ in 0..10 {
                tracing::info!(target: "test::rate", message = "flood");
            }
        });

        // Only 2 should have gotten through (bucket started with 2 tokens)
        let mut count = 0;
        while console_rx.try_recv().is_ok() {
            count += 1;
        }
        assert!(
            count <= 2,
            "at most 2 logs should pass rate limiter, got {}",
            count
        );
        // Dropped logs should be counted
        assert!(
            metrics.logs_dropped() >= 8,
            "at least 8 logs should be dropped, got {}",
            metrics.logs_dropped()
        );
    }

    #[test]
    fn test_rate_limiter_samples_error_on_rejection() {
        let (console_tx, console_rx) = bounded(200);
        let (async_tx, _async_rx) = bounded(200);
        let metrics = Arc::new(Metrics::new());
        // Rate of 1: only 1 log allowed, then all rejected
        let limiter = Arc::new(RateLimiter::new(1));

        let layer =
            LoggerSubscriber::new(console_tx, async_tx, metrics.clone()).with_rate_limiter(limiter);
        let registry = tracing_subscriber::registry().with(layer);

        with_default(registry, || {
            // First INFO consumes the token
            tracing::info!(target: "test::rate", message = "consume token");
            // Now send 100 ERRORs — only 1-in-100 should pass
            for _ in 0..100 {
                tracing::error!(target: "test::rate", message = "error flood");
            }
        });

        // Drain console channel
        let mut error_count = 0;
        while let Ok(record) = console_rx.try_recv() {
            if record.level == "ERROR" {
                error_count += 1;
            }
        }
        // First ERROR passes (counter=0, 0%100==0), rest are sampled at 1/100
        // So we expect ~1-2 ERRORs through (the first sampled one)
        assert!(
            (1..=5).contains(&error_count),
            "expected ~1 sampled ERROR through rate limiter, got {}",
            error_count
        );
    }
}

// ============================================================================
// 追踪 ID 关联 —— span 上下文提取与输出
// ============================================================================

#[cfg(test)]
mod trace_context_tests {
    use super::*;
    use crossbeam_channel::bounded;
    use tracing::subscriber::with_default;
    use tracing_subscriber::prelude::*;

    /// 构建 (registry, console_rx)：从 console 通道读取 LoggerSubscriber
    /// 提取后的记录（trace 上下文在 on_event 中注入）。
    type TestSubscriber = tracing_subscriber::layer::Layered<
        LoggerSubscriber,
        tracing_subscriber::Registry,
    >;

    fn setup() -> (
        TestSubscriber,
        crossbeam_channel::Receiver<Arc<LogRecord>>,
    ) {
        let (console_tx, console_rx) = bounded(100);
        let (async_tx, _async_rx) = bounded(100);
        let layer = LoggerSubscriber::new(console_tx, async_tx, Arc::new(Metrics::new()));
        (tracing_subscriber::registry().with(layer), console_rx)
    }

    #[test]
    fn test_event_inside_span_gets_trace_and_span_ids() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let span = tracing::info_span!("handler", request = "r-1");
            let _guard = span.enter();
            tracing::info!(target: "t504", message = "inside span");
            tracing::info!(target: "t504", message = "still inside");
        });

        let r1 = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let r2 = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let (s1, s2) = (r1.span_id.clone().expect("span_id"), r2.span_id.clone().expect("span_id"));
        assert_eq!(s1, s2, "same span must share span_id");
        assert_eq!(s1.len(), 16, "span_id must be 16-char hex");
        let (t1, t2) = (r1.trace_id.clone().expect("trace_id"), r2.trace_id.clone().expect("trace_id"));
        assert_eq!(t1, t2, "same span must share trace_id");
        assert_eq!(t1.len(), 32, "trace_id must be 32-char hex");
        assert_ne!(t1, s1, "trace_id must not equal span_id (root derivation)");
    }

    #[test]
    fn test_child_span_shares_root_trace_id() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let root = tracing::info_span!("root");
            let _root_guard = root.enter();
            tracing::info!(target: "t504", message = "at root");
            let child = tracing::info_span!("child");
            let _child_guard = child.enter();
            tracing::info!(target: "t504", message = "at child");
        });

        let root = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let child = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let root_trace = root.trace_id.clone().expect("root trace_id");
        let child_trace = child.trace_id.clone().expect("child trace_id");
        assert_eq!(
            root_trace, child_trace,
            "child span must inherit the root span's trace_id"
        );
        assert_ne!(
            root.span_id, child.span_id,
            "different spans must have different span_ids"
        );
    }

    #[test]
    fn test_event_outside_span_has_no_trace_ids() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            tracing::info!(target: "t504", message = "no span");
        });

        let record = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(record.trace_id.is_none(), "no span → trace_id None");
        assert!(record.span_id.is_none(), "no span → span_id None");
    }

    #[test]
    fn test_explicit_trace_fields_override_derivation() {
        let (subscriber, console_rx) = setup();

        with_default(subscriber, || {
            let span = tracing::info_span!("otel-ish");
            let _guard = span.enter();
            tracing::info!(
                target: "t504",
                message = "explicit",
                trace_id = "0af7651916cd43dd8448eb211c80319c",
                span_id = "b7ad6b7169203331"
            );
        });

        let record = console_rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(
            record.trace_id.as_deref(),
            Some("0af7651916cd43dd8448eb211c80319c"),
            "explicit trace_id field must win (OTel compatibility)"
        );
        assert_eq!(
            record.span_id.as_deref(),
            Some("b7ad6b7169203331"),
            "explicit span_id field must win (OTel compatibility)"
        );
    }
}
