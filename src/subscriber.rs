// Copyright (c) 2026 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::log_record::LogRecord;
use crate::metrics::Metrics;
use crate::object_pool::{LOG_RECORD_POOL, STRING_POOL};
use crossbeam_channel::Sender;
use std::sync::Arc;
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// High-performance logging subscriber with lock-free hot path.
///
/// Uses crossbeam channels for both console and async sinks to eliminate
/// lock contention in the hot path (on_event).
pub struct LoggerSubscriber {
    /// Channel sender for console output (lock-free)
    console_sender: Sender<LogRecord>,
    /// Channel sender for async sinks (file, database, etc.)
    async_sender: Sender<LogRecord>,
    /// Metrics for monitoring
    metrics: Arc<Metrics>,
}

impl LoggerSubscriber {
    pub fn new(
        console_sender: Sender<LogRecord>,
        async_sender: Sender<LogRecord>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            console_sender,
            async_sender,
            metrics,
        }
    }
}

impl<S> Layer<S> for LoggerSubscriber
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let record = LogRecord::from_event(event);

        // Fast path: Console - lock-free try_send, never block
        match self.console_sender.try_send(record.clone()) {
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

        // Slow path: Async sinks (file, database, etc.) - also never block
        match self.async_sender.try_send(record.clone()) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                // Channel full, drop the message and record metric
                self.metrics.inc_channel_blocked();
                self.metrics.inc_logs_dropped();
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                self.metrics.inc_logs_dropped();
            }
        }

        // Return resources to pools
        let mut r = record;
        let msg = std::mem::take(&mut r.message);
        STRING_POOL.put(msg);
        LOG_RECORD_POOL.put(r);
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
}
