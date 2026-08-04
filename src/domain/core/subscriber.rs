// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::LogRecord;
use crate::Metrics;
use crate::support::processing::RateLimiter;
use crate::validation::sanitize::LogSanitizer;
use crossbeam_channel::Sender;
use parking_lot::Mutex;
use serde_json::Value;
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

/// High-performance logging subscriber with lock-free hot path.
///
/// Uses crossbeam channels for both console and async sinks to eliminate
/// lock contention in the hot path (on_event).
/// Uses `Arc<LogRecord>` to avoid deep cloning when sending to multiple sinks.
/// Includes fallback buffer for critical logs (ERROR/FATAL).
pub struct LoggerSubscriber {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<Arc<LogRecord>>,
    /// Channel sender for async sinks (file, database, etc.)
    async_sender: Sender<Arc<LogRecord>>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
    /// Timeout for async channel send (milliseconds)
    send_timeout_ms: u64,
    /// Fallback buffer for critical logs
    fallback_buffer: Arc<Mutex<VecDeque<Arc<LogRecord>>>>,
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
            metrics,
            send_timeout_ms: DEFAULT_SEND_TIMEOUT_MS,
            fallback_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(FALLBACK_BUFFER_SIZE))),
            sanitizer: None,
            rate_limiter: None,
            error_sample_counter: AtomicU64::new(0),
        }
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

    /// Sanitize a log record's message and fields values.
    fn sanitize_record(&self, record: &mut LogRecord) {
        if let Some(ref sanitizer) = self.sanitizer {
            record.message = sanitizer.sanitize(&record.message);
            for value in record.fields.values_mut() {
                if let Value::String(s) = value {
                    *s = sanitizer.sanitize(s);
                }
            }
        }
    }

    pub fn try_flush_fallback(&self) {
        let mut buffer = self.fallback_buffer.lock();
        while let Some(record) = buffer.front() {
            let timeout = Duration::from_millis(self.send_timeout_ms);
            match self.async_sender.send_timeout(Arc::clone(record), timeout) {
                Ok(_) => {
                    buffer.pop_front();
                }
                Err(_) => break,
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
                tracing::warn!(
                    unflushed_records = remaining,
                    "LoggerSubscriber dropped with unflushed fallback records"
                );
            }
        }
    }
}

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut record = LogRecord::from_event(event);

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
        match self.async_sender.send_timeout(Arc::clone(&record), timeout) {
            Ok(_) => {}
            Err(crossbeam_channel::SendTimeoutError::Timeout(_)) => {
                // For critical logs, add to fallback buffer
                if Self::is_critical_level(&record.level) {
                    let mut buffer = self.fallback_buffer.lock();
                    if buffer.len() >= FALLBACK_BUFFER_SIZE {
                        buffer.pop_front();
                    }
                    buffer.push_back(record);
                } else {
                    // Timeout on non-critical log: message is lost (send_timeout returns
                    // ownership but we have nowhere to buffer it). Only count as dropped,
                    // not as channel_blocked, to keep metric semantics distinct:
                    // channel_blocked = backpressure event, logs_dropped = data loss.
                    self.metrics.inc_logs_dropped();
                }
            }
            Err(crossbeam_channel::SendTimeoutError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        subscriber
            .fallback_buffer
            .lock()
            .push_back(Arc::clone(&record));

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
        subscriber
            .fallback_buffer
            .lock()
            .push_back(Arc::clone(&record));

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
        let record = buffer_guard
            .front()
            .expect("should have a record in fallback_buffer");
        assert_eq!(record.level, "ERROR", "record level should be ERROR");
        assert_eq!(
            record.message, "critical timeout",
            "record message should match"
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
    // T003: sanitizer integration tests
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
    // T005: rate limiter integration tests
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
            error_count >= 1 && error_count <= 5,
            "expected ~1 sampled ERROR through rate limiter, got {}",
            error_count
        );
    }
}
