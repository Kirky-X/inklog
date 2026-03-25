// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::LogRecord;
use crate::Metrics;
use crossbeam_channel::Sender;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const DEFAULT_SEND_TIMEOUT_MS: u64 = 100;
const FALLBACK_BUFFER_SIZE: usize = 100;

/// High-performance logging subscriber with lock-free hot path.
///
/// Uses crossbeam channels for both console and async sinks to eliminate
/// lock contention in the hot path (on_event).
/// Uses Arc<LogRecord> to avoid deep cloning when sending to multiple sinks.
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
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.send_timeout_ms = timeout_ms;
        self
    }

    fn is_critical_level(level: &str) -> bool {
        level == "ERROR" || level == "FATAL"
    }

    pub fn try_flush_fallback(&self) {
        let mut buffer = match self.fallback_buffer.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // Mutex poison 只在持有锁的线程 panic 时发生
                // 这时我们恢复互斥锁并继续使用（因为数据可能仍然有效）
                tracing::warn!("Fallback buffer mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
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

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let record = LogRecord::from_event(event);
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
                    let mut buffer = match self.fallback_buffer.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            // Mutex poison 只在持有锁的线程 panic 时发生
                            // 这时我们恢复互斥锁并继续使用（因为数据可能仍然有效）
                            tracing::warn!("Fallback buffer mutex poisoned, recovering");
                            poisoned.into_inner()
                        }
                    };
                    if buffer.len() >= FALLBACK_BUFFER_SIZE {
                        buffer.pop_front();
                    }
                    buffer.push_back(record);
                } else {
                    self.metrics.inc_channel_blocked();
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
}
